//! Read-side queries served to the app and to agents.
//!
//! The vault holds curated concept notes only; file and symbol detail is
//! answered live from the index by the functions here.

use std::path::Path;

use cw_core::{Edge, EdgeKind, Error, Result, Symbol, VAULT_DIR, file_node};
use cw_graph::{Graph, shortest_path};
use cw_index::{Hit, Index};
use cw_vault::lifecycle::{self, NoteState};
use cw_vault::{NoteEntry, NoteKind, Origin, VaultMeta, proposal};
use serde::{Deserialize, Serialize};

use crate::analyze::DEPENDENCY_KINDS;
use crate::git::head_commit;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteInfo {
    pub id: String,
    pub path: String,
    pub title: String,
    pub kind: NoteKind,
    pub origin: Origin,
    pub state: NoteState,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteContent {
    pub info: NoteInfo,
    pub text: String,
    /// OpenUI Lang source of the note's illustration, when it has one.
    #[serde(default)]
    pub illustration: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub files: usize,
    pub symbols: usize,
    pub edges: usize,
    pub notes: usize,
    pub pending_proposals: usize,
    pub vault_commit: Option<String>,
    pub head_commit: Option<String>,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Overview {
    pub status: Status,
    /// Text of `_HOME.md`, empty before the first generation.
    pub home: String,
    pub crates: Vec<cw_parse::CrateInfo>,
}

/// A related symbol with how sure the resolver is of the relation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Related {
    pub qualified: String,
    pub kind: String,
    pub relation: String,
    pub confidence: String,
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolDetail {
    pub symbol: Symbol,
    pub members: Vec<Symbol>,
    pub depends_on: Vec<Related>,
    pub used_by: Vec<Related>,
    /// Vault paths of notes that cite the symbol's file.
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContext {
    pub path: String,
    pub language: String,
    pub lines: u32,
    pub symbols: Vec<Symbol>,
    /// Files this file imports from.
    pub imports: Vec<String>,
    /// Files that import this file.
    pub imported_by: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Context {
    File(FileContext),
    Symbol(Box<SymbolDetail>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    pub question: String,
    /// Assembled from the best-matching notes; no LLM is involved here.
    pub answer: String,
    pub sources: Vec<Hit>,
}

fn title_of(entry: &NoteEntry) -> String {
    let stem = entry.path.rsplit('/').next().unwrap_or(&entry.path);
    stem.trim_end_matches(".md").to_string()
}

fn info(entry: &NoteEntry, vault: &Path) -> NoteInfo {
    let disk = std::fs::read_to_string(vault.join(&entry.path)).ok();
    NoteInfo {
        id: entry.id.clone(),
        path: entry.path.clone(),
        title: title_of(entry),
        kind: entry.kind,
        origin: entry.origin,
        state: lifecycle::state(entry, disk.as_deref(), false),
        sources: entry.sources.iter().map(|s| s.path.clone()).collect(),
    }
}

pub fn list_notes(root: &Path) -> Result<Vec<NoteInfo>> {
    let vault = root.join(VAULT_DIR);
    let meta = VaultMeta::load(&vault)?;
    let mut notes: Vec<NoteInfo> = meta.notes.values().map(|e| info(e, &vault)).collect();
    notes.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(notes)
}

/// Reject vault paths that could escape the vault folder.
fn check_vault_path(path: &str) -> Result<()> {
    let bad = path.is_empty()
        || path.contains('\\')
        || path.contains(':')
        || path.starts_with('/')
        || path
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == ".." || s.starts_with('.'));
    if bad || !path.ends_with(".md") {
        return Err(Error::invalid(format!("invalid note path '{path}'")));
    }
    Ok(())
}

/// Read a note by vault path or by name.
pub fn read_note(root: &Path, path_or_name: &str) -> Result<NoteContent> {
    let vault = root.join(VAULT_DIR);
    let meta = VaultMeta::load(&vault)?;
    let entry = meta
        .by_path(path_or_name)
        .or_else(|| {
            let wanted = path_or_name.trim_end_matches(".md").to_lowercase();
            meta.notes
                .values()
                .find(|e| title_of(e).to_lowercase() == wanted)
        })
        .ok_or_else(|| Error::not_found(format!("note '{path_or_name}'")))?;
    check_vault_path(&entry.path)?;
    let full = vault.join(&entry.path);
    let text = std::fs::read_to_string(&full).map_err(|e| Error::io(full, e))?;
    let illustration =
        std::fs::read_to_string(vault.join(crate::generate::illustration_path(&entry.path))).ok();
    Ok(NoteContent {
        info: info(entry, &vault),
        text,
        illustration,
    })
}

/// Save a human edit to an existing note. The manifest is left alone, so the
/// note's changed hash marks it as human-edited from now on.
pub fn write_note(root: &Path, path: &str, text: &str) -> Result<()> {
    check_vault_path(path)?;
    let vault = root.join(VAULT_DIR);
    let meta = VaultMeta::load(&vault)?;
    if meta.by_path(path).is_none() {
        return Err(Error::not_found(format!("note '{path}'")));
    }
    let full = vault.join(path);
    std::fs::write(&full, text).map_err(|e| Error::io(full, e))
}

pub fn status(root: &Path, index: &Index) -> Result<Status> {
    let vault = root.join(VAULT_DIR);
    let meta = VaultMeta::load(&vault)?;
    let head = head_commit(root);
    let pending = proposal::list(&vault)?
        .iter()
        .filter(|p| p.status == proposal::ProposalStatus::Pending)
        .count();
    Ok(Status {
        files: index.files()?.len(),
        symbols: index.symbols()?.len(),
        edges: index.edges()?.len(),
        notes: meta.notes.len(),
        pending_proposals: pending,
        stale: crate::git::vault_is_behind(root, meta.manifest.last_indexed_commit.as_deref()),
        vault_commit: meta.manifest.last_indexed_commit,
        head_commit: head,
    })
}

pub fn overview(root: &Path, index: &Index) -> Result<Overview> {
    let home = std::fs::read_to_string(root.join(VAULT_DIR).join("_HOME.md")).unwrap_or_default();
    Ok(Overview {
        status: status(root, index)?,
        home,
        crates: index.crates()?,
    })
}

fn related(index: &Index, edges: Vec<Edge>, outgoing: bool) -> Result<Vec<Related>> {
    let mut out = Vec::new();
    for e in edges
        .into_iter()
        .filter(|e| DEPENDENCY_KINDS.contains(&e.kind))
    {
        let other = if outgoing { &e.dst } else { &e.src };
        if let Some(symbol) = index.symbol(other)? {
            out.push(Related {
                qualified: symbol.qualified,
                kind: symbol.kind.as_str().to_string(),
                relation: e.kind.as_str().to_string(),
                confidence: e.confidence.as_str().to_string(),
                file: symbol.file,
            });
        }
    }
    Ok(out)
}

fn notes_citing(root: &Path, file: &str) -> Result<Vec<String>> {
    let meta = VaultMeta::load(&root.join(VAULT_DIR))?;
    let mut notes: Vec<String> = meta
        .notes_for_source(file)
        .iter()
        .map(|e| e.path.clone())
        .collect();
    notes.sort();
    Ok(notes)
}

/// Detail for one symbol, by qualified or bare name.
pub fn symbol_detail(root: &Path, index: &Index, name: &str) -> Result<SymbolDetail> {
    let symbol = match index.symbol(name)? {
        Some(s) => s,
        None => index
            .find_symbols(name, 1)?
            .into_iter()
            .next()
            .ok_or_else(|| Error::not_found(format!("symbol '{name}'")))?,
    };
    // A type's relations include those of its methods.
    let members: Vec<Symbol> = index
        .symbols_in_file(&symbol.file)?
        .into_iter()
        .filter(|s| s.parent.as_deref() == Some(symbol.qualified.as_str()))
        .collect();
    let mut depends_on = related(index, index.edges_from(&symbol.qualified)?, true)?;
    let mut used_by = related(index, index.edges_to(&symbol.qualified)?, false)?;
    for member in &members {
        depends_on.extend(related(index, index.edges_from(&member.qualified)?, true)?);
        used_by.extend(related(index, index.edges_to(&member.qualified)?, false)?);
    }
    let own = |r: &Related| {
        r.qualified == symbol.qualified
            || r.qualified.starts_with(&format!("{}::", symbol.qualified))
    };
    for list in [&mut depends_on, &mut used_by] {
        list.retain(|r| !own(r));
        list.sort_by(|a, b| {
            a.qualified
                .cmp(&b.qualified)
                .then(a.relation.cmp(&b.relation))
        });
        list.dedup_by(|a, b| a.qualified == b.qualified && a.relation == b.relation);
    }
    Ok(SymbolDetail {
        notes: notes_citing(root, &symbol.file)?,
        symbol,
        members,
        depends_on,
        used_by,
    })
}

/// Context for a file path or a symbol name: what an agent should know before
/// touching it.
pub fn context(root: &Path, index: &Index, target: &str) -> Result<Context> {
    let normalized = target.replace('\\', "/");
    let files = index.files()?;
    if let Some(file) = files.iter().find(|f| f.path == normalized) {
        let node = file_node(&file.path);
        let strip = |n: &str| n.strip_prefix("file:").unwrap_or(n).to_string();
        let imports = index
            .edges_from(&node)?
            .iter()
            .filter(|e| e.kind == EdgeKind::Imports)
            .map(|e| strip(&e.dst))
            .collect();
        let imported_by = index
            .edges_to(&node)?
            .iter()
            .filter(|e| e.kind == EdgeKind::Imports)
            .map(|e| strip(&e.src))
            .collect();
        return Ok(Context::File(FileContext {
            path: file.path.clone(),
            language: file.language.as_str().to_string(),
            lines: file.lines,
            symbols: index.symbols_in_file(&file.path)?,
            imports,
            imported_by,
            notes: notes_citing(root, &file.path)?,
        }));
    }
    symbol_detail(root, index, target).map(|d| Context::Symbol(Box::new(d)))
}

/// Shortest dependency chain between two symbols, by qualified or bare name.
pub fn dependency_path(index: &Index, from: &str, to: &str) -> Result<Vec<String>> {
    let resolve = |name: &str| -> Result<String> {
        if let Some(s) = index.symbol(name)? {
            return Ok(s.qualified);
        }
        index
            .find_symbols(name, 1)?
            .into_iter()
            .next()
            .map(|s| s.qualified)
            .ok_or_else(|| Error::not_found(format!("symbol '{name}'")))
    };
    let (from, to) = (resolve(from)?, resolve(to)?);
    let edges = index.edges()?;
    let graph = Graph::from_edges(Vec::new(), &edges, &DEPENDENCY_KINDS);
    let (Some(a), Some(b)) = (graph.id(&from), graph.id(&to)) else {
        return Ok(Vec::new());
    };
    Ok(shortest_path(&graph, a, b)
        .map(|path| path.into_iter().map(|i| graph.nodes[i].clone()).collect())
        .unwrap_or_default())
}

/// Answer a question from the vault without an LLM: the best-matching notes'
/// summaries, with their sources. An LLM-backed answer can build on the same
/// retrieval.
pub fn answer(root: &Path, index: &Index, question: &str) -> Result<Answer> {
    let mut sources = index.search(question, Some("note"), 4)?;
    sources.extend(index.search(question, Some("symbol"), 4)?);
    let vault = root.join(VAULT_DIR);
    let mut parts = Vec::new();
    for hit in sources.iter().filter(|h| h.kind == "note").take(3) {
        if let Ok(text) = std::fs::read_to_string(vault.join(&hit.key)) {
            let note = cw_vault::Note::parse(&text);
            if let Some(summary) = note.summary {
                parts.push(format!(
                    "{} — {summary} (see `{}/{}`)",
                    hit.title, VAULT_DIR, hit.key
                ));
            }
        }
    }
    let answer = if parts.is_empty() {
        "No note in the vault matches this question. Try `search` with different words, or `context` on a file or symbol.".to_string()
    } else {
        parts.join("\n")
    };
    Ok(Answer {
        question: question.to_string(),
        answer,
        sources,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_vault_paths() {
        for bad in [
            "../x.md",
            "a/../b.md",
            "/abs.md",
            "C:/x.md",
            "a\\b.md",
            ".codewiki/x.md",
            "a/b.txt",
            "",
        ] {
            assert!(check_vault_path(bad).is_err(), "{bad}");
        }
        assert!(check_vault_path("01-Core/engine/Price.md").is_ok());
        assert!(check_vault_path("_HOME.md").is_ok());
    }
}
