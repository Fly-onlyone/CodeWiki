//! The generation pipeline: scan, plan, freeze names, render, lint, write.
//!
//! One function serves first-time generation, incremental updates and
//! adoption of an existing vault. Whether a note is written, proposed or left
//! alone is decided per note by [`cw_vault::lifecycle`].

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use cw_core::{Error, Result, VAULT_DIR, sha256_hex};
use cw_index::NodeMetrics;
use cw_parse::RepoFacts;
use cw_vault::lifecycle::{self, Action, NoteState};
use cw_vault::lint::{Mode, Severity, link_resolution_rate, lint};
use cw_vault::{
    Note, NoteEntry, NoteKind, Origin, Registry, RegistryEntry, SourceRef, VaultMeta, body_hash,
    proposal,
};
use serde::{Deserialize, Serialize};

use crate::plan::{Plan, Planned, Repo, plan};
use crate::render::RenderCtx;

/// Bump when rendering changes in a way that should refresh every note.
pub const TEMPLATE_VERSION: u32 = 1;

/// Replaces the structural prose of a note, typically with an LLM. Returning
/// `None` keeps the structural draft.
pub trait ProseWriter {
    /// Identifier recorded in the manifest, for example a model name.
    fn name(&self) -> String;
    fn rewrite(&self, note: &Planned, draft: &str, context: &WriterContext) -> Option<String>;
    /// An illustration for the finished note, as OpenUI Lang source, or
    /// `None` to keep the structural one. `structural` is the illustration
    /// built from facts alone, when the note has one. The result is checked
    /// against the component library before it is saved.
    fn illustrate(
        &self,
        _note: &Planned,
        _text: &str,
        _structural: Option<&str>,
        _context: &WriterContext,
    ) -> Option<String> {
        None
    }
    /// Calls and tokens spent so far; reported after each run.
    fn usage(&self) -> WriterUsage {
        WriterUsage::default()
    }
}

/// What a writer spent during one run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriterUsage {
    pub calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// Calls whose output was rejected or failed; the structural draft was used.
    pub fallbacks: u64,
}

/// Facts handed to a [`ProseWriter`].
pub struct WriterContext<'a> {
    pub repo_root: &'a Path,
    /// Every note name that may be linked.
    pub note_names: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub written: Vec<String>,
    pub proposed: Vec<String>,
    pub removed: Vec<String>,
    pub adopted: Vec<String>,
    /// Illustration files written this run.
    #[serde(default)]
    pub illustrated: Vec<String>,
    /// Notes left unmanaged because another note already has the same name.
    #[serde(default)]
    pub skipped: Vec<String>,
    pub kept: usize,
    pub notes_total: usize,
    pub lint_warnings: usize,
    /// Share of wikilinks in CodeWiki-written notes that resolve.
    pub link_rate: f64,
    /// LLM spend of this run; all zero in keyless mode.
    #[serde(default)]
    pub llm: WriterUsage,
    pub commit: Option<String>,
}

fn read_optional(path: &Path) -> Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::io(path, e)),
    }
}

/// Vault-relative paths of every Markdown note on disk.
fn list_notes(vault: &Path) -> Result<Vec<String>> {
    fn walk(dir: &Path, vault: &Path, out: &mut Vec<String>) -> Result<()> {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(Error::io(dir, e)),
        };
        for entry in entries {
            let entry = entry.map_err(|e| Error::io(dir, e))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if !name.starts_with('.') && name != "_Templates" && name != "_Attachments" {
                    walk(&path, vault, out)?;
                }
            } else if name.ends_with(".md")
                && let Ok(rel) = path.strip_prefix(vault)
            {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(vault, vault, &mut out)?;
    out.sort();
    Ok(out)
}

/// Kind of a note that CodeWiki did not write, from its name and tags.
fn adopted_kind(path: &str, note: &Note) -> NoteKind {
    let file = path.rsplit('/').next().unwrap_or(path);
    if let Some(kind) = NoteKind::from_file_name(file) {
        return kind;
    }
    let has = |tag: &str| note.tags.iter().any(|t| t == tag);
    if has("pattern") {
        NoteKind::Pattern
    } else if has("integration") || has("integrations") {
        NoteKind::Integration
    } else if has("decision") || has("adr") {
        NoteKind::Decision
    } else if has("glossary") {
        NoteKind::Glossary
    } else if has("operations") {
        NoteKind::Operations
    } else if has("flow") {
        NoteKind::Flow
    } else {
        NoteKind::Concept
    }
}

/// Source files named in a note's `## Source` section that exist in the repo.
fn cited_sources(note: &Note, file_hashes: &HashMap<&str, &str>) -> Vec<SourceRef> {
    let Some(section) = note.section("Source") else {
        return Vec::new();
    };
    let mut out: Vec<SourceRef> = Vec::new();
    for span in section.body.split('`').skip(1).step_by(2) {
        let path = span.trim().trim_end_matches('/');
        if let Some(hash) = file_hashes.get(path)
            && !out.iter().any(|s| s.path == path)
        {
            out.push(SourceRef {
                path: path.to_string(),
                symbols: Vec::new(),
                hash: hash.to_string(),
            });
        }
    }
    out
}

/// Register notes found on disk that the manifest does not know.
fn adopt_existing(
    vault: &Path,
    meta: &mut VaultMeta,
    facts: &RepoFacts,
    report: &mut Report,
) -> Result<()> {
    let file_hashes: HashMap<&str, &str> = facts
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.hash.as_str()))
        .collect();
    let known: HashSet<String> = meta.notes.values().map(|e| e.path.to_lowercase()).collect();
    // Wikilinks name notes by title, so two notes cannot share one. A
    // hand-written vault may contain such a pair; the second is left unmanaged
    // and reported rather than failing the whole run.
    let stem_of = |path: &str| {
        let file = path.rsplit('/').next().unwrap_or(path);
        file.trim_end_matches(".md").to_lowercase()
    };
    let mut names: HashSet<String> = meta
        .notes
        .values()
        .filter(|e| !matches!(e.kind, NoteKind::Home | NoteKind::Index))
        .map(|e| stem_of(&e.path))
        .collect();
    for path in list_notes(vault)? {
        if path == "README.md" || known.contains(&path.to_lowercase()) {
            continue;
        }
        let file = path.rsplit('/').next().unwrap_or(&path);
        if NoteKind::from_file_name(file).is_none() && !names.insert(stem_of(&path)) {
            report.skipped.push(path);
            continue;
        }
        let text = std::fs::read_to_string(vault.join(&path))
            .map_err(|e| Error::io(vault.join(&path), e))?;
        let note = Note::parse(&text);
        let entry = NoteEntry {
            id: format!("adopted:{path}"),
            path: path.clone(),
            kind: adopted_kind(&path, &note),
            origin: Origin::Adopted,
            sources: cited_sources(&note, &file_hashes),
            body_hash: body_hash(&text),
            // Empty until a planned note is first matched against it.
            input_hash: String::new(),
            rejected_inputs: Vec::new(),
            commit: None,
            model: None,
            illustration: None,
        };
        report.adopted.push(path);
        meta.notes.insert(entry.id.clone(), entry);
    }
    Ok(())
}

fn planned_path(note: &Planned) -> String {
    let file = format!("{}.md", cw_vault::sanitize_name(&note.title));
    let file = match note.kind {
        NoteKind::Home => "_HOME.md".to_string(),
        NoteKind::Index => "_index.md".to_string(),
        _ => file,
    };
    if note.dir.is_empty() {
        file
    } else {
        format!("{}/{file}", note.dir)
    }
}

/// Generate or update the vault of the repository at `root`.
pub fn generate(
    root: &Path,
    facts: &RepoFacts,
    metrics: &[(String, NodeMetrics)],
    commit: Option<String>,
    writer: Option<&dyn ProseWriter>,
) -> Result<Report> {
    let vault = root.join(VAULT_DIR);
    std::fs::create_dir_all(&vault).map_err(|e| Error::io(&vault, e))?;
    let mut meta = VaultMeta::load(&vault)?;
    let mut report = Report {
        commit: commit.clone(),
        ..Report::default()
    };
    adopt_existing(&vault, &mut meta, facts, &mut report)?;

    // Files documented by adopted notes get no new module or type notes.
    let mut covered: HashMap<String, String> = HashMap::new();
    for entry in meta.notes.values().filter(|e| e.origin == Origin::Adopted) {
        for source in &entry.sources {
            covered
                .entry(source.path.clone())
                .or_insert_with(|| entry.id.clone());
        }
    }

    let repo = Repo::at(root, facts, metrics);
    let Plan { mut notes, owner } = plan(&repo, &covered);

    // Freeze names. Existing notes keep their paths; a planned note that
    // collides with an adopted note by path or name takes over its entry.
    let mut by_path: HashMap<String, String> = meta
        .notes
        .values()
        .map(|e| (e.path.to_lowercase(), e.id.clone()))
        .collect();
    let mut by_name: HashMap<String, String> = meta
        .notes
        .values()
        .filter(|e| !matches!(e.kind, NoteKind::Home | NoteKind::Index))
        .map(|e| {
            let stem = e.path.rsplit('/').next().unwrap_or(&e.path);
            (stem.trim_end_matches(".md").to_lowercase(), e.id.clone())
        })
        .collect();
    for note in &notes {
        if meta.notes.contains_key(&note.id) {
            continue;
        }
        let path = planned_path(note);
        let name = cw_vault::sanitize_name(&note.title).to_lowercase();
        let taken = by_path.get(&path.to_lowercase()).or_else(|| {
            (!matches!(note.kind, NoteKind::Home | NoteKind::Index))
                .then(|| by_name.get(&name))
                .flatten()
        });
        // Only an adopted note is taken over. When a generated note with
        // another id owns the name (for example a type that moved between
        // crates), its entry must stay so the stale file is cleaned up below;
        // the registry then picks a different name for the new note.
        if let Some(old_id) = taken.cloned()
            && meta
                .notes
                .get(&old_id)
                .is_some_and(|e| e.origin == Origin::Adopted)
            && let Some(mut entry) = meta.notes.remove(&old_id)
        {
            by_path.insert(entry.path.to_lowercase(), note.id.clone());
            by_name.insert(name, note.id.clone());
            entry.id = note.id.clone();
            meta.notes.insert(entry.id.clone(), entry);
        }
    }

    let mut registry = Registry::new();
    for entry in meta.notes.values() {
        registry.insert(RegistryEntry {
            id: entry.id.clone(),
            path: entry.path.clone(),
            kind: entry.kind,
        })?;
    }
    let mut paths: HashMap<String, String> = HashMap::new();
    for note in &notes {
        let path = match meta.notes.get(&note.id) {
            Some(entry) => entry.path.clone(),
            None => match note.kind {
                NoteKind::Home | NoteKind::Index => {
                    let path = planned_path(note);
                    registry.insert(RegistryEntry {
                        id: note.id.clone(),
                        path: path.clone(),
                        kind: note.kind,
                    })?;
                    path
                }
                _ => {
                    registry
                        .reserve(&note.id, &note.dir, &note.title, note.kind)
                        .path
                }
            },
        };
        paths.insert(note.id.clone(), path);
    }
    // Titles follow the reserved file names so `# Title` matches the link.
    for note in &mut notes {
        if !matches!(note.kind, NoteKind::Home | NoteKind::Index) {
            let path = &paths[&note.id];
            let stem = path.rsplit('/').next().unwrap_or(path);
            note.title = stem.trim_end_matches(".md").to_string();
        }
    }

    let repo_name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Repository".into());
    let plan = Plan { notes, owner };
    let ctx = RenderCtx::new(&repo, &plan, &registry, &repo_name);
    let writer_context = WriterContext {
        repo_root: root,
        note_names: registry
            .entries()
            .filter(|e| !matches!(e.kind, NoteKind::Home | NoteKind::Index))
            .map(|e| {
                let stem = e.path.rsplit('/').next().unwrap_or(&e.path);
                stem.trim_end_matches(".md").to_string()
            })
            .collect(),
    };
    let file_hashes: HashMap<&str, &str> = facts
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.hash.as_str()))
        .collect();

    // Structural drafts must pass the linter on their own: they are both the
    // keyless output and the fallback when a writer's prose is rejected.
    let mut drafts: BTreeMap<String, String> = BTreeMap::new();
    let mut errors: Vec<String> = Vec::new();
    for note in &plan.notes {
        let path = &paths[&note.id];
        let draft = ctx.render(note);
        for issue in lint(&draft, path, note.kind, &registry, Mode::Strict) {
            match issue.severity {
                Severity::Error => {
                    errors.push(format!("{path}: [{}] {}", issue.rule, issue.message))
                }
                Severity::Warning => report.lint_warnings += 1,
            }
        }
        drafts.insert(note.id.clone(), draft);
    }
    if !errors.is_empty() {
        errors.truncate(20);
        return Err(Error::invalid(format!(
            "generated notes failed the style linter:\n{}",
            errors.join("\n")
        )));
    }

    let writer_name = writer.map(|w| w.name());
    let is_map = |note: &Planned| matches!(note.kind, NoteKind::Home | NoteKind::Index);
    // Called only for notes that are about to be written or proposed, so an
    // unchanged repository costs no LLM calls.
    let finalize = |note: &Planned, path: &str, draft: &str| -> (String, Option<String>) {
        if let Some(writer) = writer
            && !is_map(note)
            && let Some(candidate) = writer.rewrite(note, draft, &writer_context)
        {
            // Prose that breaks the house style is discarded, not repaired by hand.
            let clean = lint(&candidate, path, note.kind, &registry, Mode::Strict)
                .iter()
                .all(|i| i.severity != Severity::Error);
            if clean {
                return (candidate, writer_name.clone());
            }
        }
        (draft.to_string(), None)
    };

    for note in &plan.notes {
        let path = &paths[&note.id];
        let draft = &drafts[&note.id];
        let full = vault.join(path);
        let disk = read_optional(&full)?;
        let sources: Vec<SourceRef> = note
            .sources
            .iter()
            .filter_map(|p| {
                Some(SourceRef {
                    path: p.clone(),
                    symbols: Vec::new(),
                    hash: file_hashes.get(p.as_str())?.to_string(),
                })
            })
            .collect();
        // The hash covers generation inputs, never the output, so a writer
        // that phrases things differently each run does not cause churn.
        let mut hash_input = format!("v{TEMPLATE_VERSION}\n{draft}");
        if let Some(name) = &writer_name
            && !is_map(note)
        {
            // A writer reads function bodies, so edits there matter even when
            // the structural draft is unchanged.
            hash_input.push_str(&format!("\nwriter:{name}"));
            for source in &sources {
                hash_input.push_str(&format!("\n{}:{}", source.path, source.hash));
            }
        }
        let input_hash = sha256_hex(hash_input);
        let fresh_entry = |text: &str, model: Option<String>| NoteEntry {
            id: note.id.clone(),
            path: path.clone(),
            kind: note.kind,
            origin: Origin::Generated,
            sources: sources.clone(),
            body_hash: body_hash(text),
            input_hash: input_hash.clone(),
            rejected_inputs: Vec::new(),
            commit: commit.clone(),
            model,
            illustration: None,
        };

        let Some(entry) = meta.notes.get_mut(&note.id) else {
            let (text, model) = finalize(note, path, draft);
            write_note(&full, &text)?;
            report.written.push(path.clone());
            meta.notes
                .insert(note.id.clone(), fresh_entry(&text, model));
            continue;
        };
        // First contact with an adopted note: record what a regeneration
        // would say now, so only later source changes raise a proposal.
        if entry.origin == Origin::Adopted && entry.input_hash.is_empty() {
            entry.input_hash = input_hash;
            if entry.sources.is_empty() {
                entry.sources = sources;
            }
            report.kept += 1;
            continue;
        }
        let changed = entry.input_hash != input_hash;
        let state = lifecycle::state(entry, disk.as_deref(), changed);
        match lifecycle::action(entry, state, &input_hash) {
            Action::Keep => report.kept += 1,
            Action::Regenerate => {
                let (text, model) = finalize(note, path, draft);
                write_note(&full, &text)?;
                report.written.push(path.clone());
                let rejected = std::mem::take(&mut entry.rejected_inputs);
                *entry = fresh_entry(&text, model);
                entry.rejected_inputs = rejected;
            }
            Action::Propose => {
                debug_assert_eq!(state, NoteState::HumanEditedStale);
                let current = disk.unwrap_or_default();
                let pending_same = proposal::list(&vault)?.iter().any(|p| {
                    p.note_id == note.id
                        && p.input_hash == input_hash
                        && p.status == proposal::ProposalStatus::Pending
                });
                if !pending_same {
                    let (text, _) = finalize(note, path, draft);
                    proposal::create(
                        &vault,
                        &note.id,
                        path,
                        &current,
                        &text,
                        &input_hash,
                        "cited source code changed",
                    )?;
                }
                report.proposed.push(path.clone());
            }
        }
    }
    // Illustrations sit beside their note as `<Note>.openui`. They follow the
    // note's ownership: only notes CodeWiki still owns get theirs refreshed.
    for note in &plan.notes {
        // An illustration that fails its own library check is never saved.
        let structural = ctx
            .illustration(note)
            .filter(|source| cw_illustrate::check(source).is_valid());
        let drawable = structural.is_some() || note.kind == NoteKind::Flow;
        if !drawable {
            continue;
        }
        let Some(entry) = meta.notes.get_mut(&note.id) else {
            continue;
        };
        let note_text = read_optional(&vault.join(&entry.path))?;
        let Some(note_text) = note_text
            .filter(|t| entry.origin == Origin::Generated && body_hash(t) == entry.body_hash)
        else {
            continue;
        };
        let relative = illustration_path(&entry.path);
        let full = vault.join(&relative);
        let existing = read_optional(&full)?;
        let source = match writer {
            // A writer draws only when the note changed in this run or has no
            // illustration yet; otherwise every sync would pay for a redraw.
            Some(writer) if report.written.contains(&entry.path) || existing.is_none() => writer
                .illustrate(note, &note_text, structural.as_deref(), &writer_context)
                .filter(|source| cw_illustrate::check(source).is_valid())
                .or(structural),
            // Keep what an earlier run with a writer drew.
            Some(_) => existing
                .clone()
                .filter(|source| cw_illustrate::check(source).is_valid())
                .or(structural),
            None => structural,
        };
        let Some(source) = source else {
            continue;
        };
        if existing.as_deref() != Some(source.as_str()) {
            write_note(&full, &source)?;
            report.illustrated.push(relative.clone());
        }
        entry.illustration = Some(relative);
    }
    if let Some(writer) = writer {
        report.llm = writer.usage();
    }

    // Notes CodeWiki wrote whose subject no longer exists.
    let planned_ids: HashSet<&str> = plan.notes.iter().map(|n| n.id.as_str()).collect();
    let stale: Vec<String> = meta
        .notes
        .values()
        .filter(|e| e.origin == Origin::Generated && !planned_ids.contains(e.id.as_str()))
        .map(|e| e.id.clone())
        .collect();
    for id in stale {
        let entry = &meta.notes[&id];
        let full = vault.join(&entry.path);
        let disk = read_optional(&full)?;
        let untouched = disk
            .as_deref()
            .is_none_or(|t| body_hash(t) == entry.body_hash);
        if untouched {
            if disk.is_some() {
                std::fs::remove_file(&full).map_err(|e| Error::io(&full, e))?;
            }
            // The illustration has no meaning without its note.
            let _ = std::fs::remove_file(vault.join(illustration_path(&entry.path)));
            report.removed.push(entry.path.clone());
            meta.notes.remove(&id);
        }
    }

    write_readme(&vault, &repo_name)?;
    meta.manifest.template_version = TEMPLATE_VERSION;
    meta.manifest.last_indexed_commit = commit;
    meta.save(&vault)?;

    let final_registry = meta.registry()?;
    let own: Vec<(String, String)> = meta
        .notes
        .values()
        .filter(|e| e.origin == Origin::Generated)
        .filter_map(|e| Some((e.path.clone(), read_optional(&vault.join(&e.path)).ok()??)))
        .collect();
    report.link_rate = link_resolution_rate(
        own.iter().map(|(p, t)| (p.as_str(), t.as_str())),
        &final_registry,
    );
    report.notes_total = meta.notes.len();
    Ok(report)
}

/// Vault path of the illustration that belongs to the note at `note_path`.
pub fn illustration_path(note_path: &str) -> String {
    format!(
        "{}.{}",
        note_path.strip_suffix(".md").unwrap_or(note_path),
        cw_illustrate::EXTENSION
    )
}

fn write_note(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    std::fs::write(path, text).map_err(|e| Error::io(path, e))
}

/// The vault's own README explains how to open it; written once, never updated.
fn write_readme(vault: &Path, repo_name: &str) -> Result<()> {
    let path = vault.join("README.md");
    if path.exists() {
        return Ok(());
    }
    let text = format!(
        "# {repo_name} — Obsidian vault\n\nThis folder is an Obsidian vault maintained by CodeWiki.\n\n1. In Obsidian choose **Open folder as vault** and select this folder.\n2. Start at `_HOME.md`.\n\nNotes you edit are never overwritten: CodeWiki queues a proposal for your review instead. Machine metadata lives in `.codewiki/` and should be committed with the notes.\n"
    );
    std::fs::write(&path, text).map_err(|e| Error::io(path, e))
}
