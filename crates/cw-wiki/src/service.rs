//! High-level operations shared by the CLI, the server and the MCP tools.

use std::path::Path;

use cw_core::paths::{index_path, index_path_in};
use cw_core::{Result, VAULT_DIR};
use cw_index::{Document, Index};
use cw_parse::{RepoFacts, scan_repo};
use cw_vault::{Note, VaultMeta};

use crate::analyze::analyze;
use crate::generate::{ProseWriter, Report, generate};
use crate::git::head_commit;

/// Meta key holding the commit the index was last built from.
pub const META_COMMIT: &str = "indexed_commit";

/// Open the index of the repository at `root` in the default data directory.
pub fn open_index(root: &Path) -> Result<Index> {
    built(root, Index::open(&index_path(root))?)
}

/// Open the index of the repository at `root` under `data_dir`.
pub fn open_index_in(data_dir: &Path, root: &Path) -> Result<Index> {
    built(root, Index::open(&index_path_in(data_dir, root))?)
}

/// Meta key set once the index has been filled from the repository.
const META_BUILT: &str = "built";

/// Fill an index that has never been built. The index is a cache: a new data
/// directory, or a schema change, leaves it empty while the vault already
/// exists, and every query would then answer as if the repository were empty.
fn built(root: &Path, mut index: Index) -> Result<Index> {
    if index.meta(META_BUILT)?.is_none() {
        reindex(root, &mut index)?;
        if root.join(VAULT_DIR).is_dir() {
            index_notes(root, &mut index)?;
        }
    }
    Ok(index)
}

/// Scan the repository and rewrite the structural index. Notes are not touched.
pub fn reindex(root: &Path, index: &mut Index) -> Result<RepoFacts> {
    let facts = scan_repo(root)?;
    let metrics = analyze(&facts);
    index.replace_facts(&facts)?;
    index.replace_metrics(&metrics)?;
    index.set_meta(META_BUILT, "1")?;
    let documents: Vec<Document> = facts
        .symbols
        .iter()
        .map(|s| Document {
            kind: "symbol".into(),
            key: s.qualified.clone(),
            title: s.name.clone(),
            body: format!("{}\n{}\n{}", s.qualified, s.signature, s.doc),
        })
        .chain(facts.files.iter().map(|f| Document {
            kind: "file".into(),
            key: f.path.clone(),
            title: f.path.rsplit('/').next().unwrap_or(&f.path).to_string(),
            body: f.path.replace(['/', '.', '-'], " "),
        }))
        .collect();
    index.replace_documents(&["symbol", "file"], &documents)?;
    crate::semantic::refresh(index, &["symbol", "file"], &documents);
    index.set_meta(META_COMMIT, head_commit(root).as_deref().unwrap_or(""))?;
    crate::gitintel::save(index, &crate::gitintel::mine(root))?;
    Ok(facts)
}

/// Load every vault note into the search index.
pub fn index_notes(root: &Path, index: &mut Index) -> Result<usize> {
    let vault = root.join(VAULT_DIR);
    let meta = VaultMeta::load(&vault)?;
    let mut documents = Vec::new();
    for entry in meta.notes.values() {
        let Ok(text) = std::fs::read_to_string(vault.join(&entry.path)) else {
            continue;
        };
        let note = Note::parse(&text);
        let stem = entry.path.rsplit('/').next().unwrap_or(&entry.path);
        documents.push(Document {
            kind: "note".into(),
            key: entry.path.clone(),
            title: note
                .title
                .unwrap_or_else(|| stem.trim_end_matches(".md").to_string()),
            body: text,
        });
    }
    index.replace_documents(&["note"], &documents)?;
    crate::semantic::refresh(index, &["note"], &documents);
    Ok(documents.len())
}

/// Bring index and vault up to date with the working tree.
pub fn update(root: &Path, writer: Option<&dyn ProseWriter>) -> Result<Report> {
    update_in(&cw_core::paths::data_dir(), root, writer)
}

/// [`update`] with an explicit data directory.
pub fn update_in(data_dir: &Path, root: &Path, writer: Option<&dyn ProseWriter>) -> Result<Report> {
    let mut index = open_index_in(data_dir, root)?;
    let facts = reindex(root, &mut index)?;
    let metrics = index.metrics()?;
    let report = generate(root, &facts, &metrics, head_commit(root), writer)?;
    index_notes(root, &mut index)?;
    Ok(report)
}

/// True when `HEAD` differs from the commit the vault was last generated at.
pub fn is_stale(root: &Path) -> Result<bool> {
    let meta = VaultMeta::load(&root.join(VAULT_DIR))?;
    Ok(meta.manifest.last_indexed_commit != head_commit(root))
}
