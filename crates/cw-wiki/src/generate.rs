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
    Gone, Note, NoteEntry, NoteKind, Origin, Registry, RegistryEntry, SourceRef, VaultMeta,
    body_hash, proposal,
};
use serde::{Deserialize, Serialize};

use crate::gone::{self, Verdict};
use crate::manifests::{self, NoPackageInfo, PackageInfo};
use crate::plan::{Plan, Planned, Repo, Subject, plan};
use crate::render::{HandZone, RenderCtx};

/// Bump when rendering changes in a way that should refresh every note.
pub const TEMPLATE_VERSION: u32 = 2;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub written: Vec<String>,
    pub proposed: Vec<String>,
    pub removed: Vec<String>,
    pub adopted: Vec<String>,
    /// Notes left unmanaged because another note already has the same name.
    #[serde(default)]
    pub skipped: Vec<String>,
    /// Notes an agent explained whose facts changed: rules wrote their own
    /// sections anew and the agent's sections stayed.
    #[serde(default)]
    pub merged: Vec<String>,
    /// Notes whose explanation may be behind the code: a file it was
    /// written from changed since.
    #[serde(default)]
    pub behind: Vec<String>,
    /// Notes that stay although what they describe is gone, in the order
    /// of their paths.
    #[serde(default)]
    pub kept_gone: Vec<KeptGone>,
    /// Notes that followed what they describe to a new place, with all
    /// that was written in them.
    #[serde(default)]
    pub moved: Vec<Moved>,
    /// Notes with written work whose subject may have moved, when the run
    /// could not tell where to: nothing was carried over, and each is
    /// treated as a note of something gone.
    #[serde(default)]
    pub possible_moves: Vec<String>,
    /// Notes their owner wrote that name only paths that are gone. They
    /// wait in one retire proposal for a decision; none is deleted.
    #[serde(default)]
    pub retire: Vec<String>,
    /// Notes their owner wrote that name paths that are gone beside paths
    /// that exist. They stay as they are.
    #[serde(default)]
    pub partly_gone: Vec<String>,
    /// Notes their owner wrote in which the path of a moved file was
    /// replaced by where the file is now.
    #[serde(default)]
    pub repathed: Vec<String>,
    /// Notes that stay in which a link to a removed note became plain
    /// words.
    #[serde(default)]
    pub unlinked: Vec<String>,
    /// Attachments deleted because only removed notes showed them.
    #[serde(default)]
    pub attachments_removed: Vec<String>,
    /// Notes with a proposal that waited and can no longer be approved,
    /// because what the note describes went.
    #[serde(default)]
    pub superseded: Vec<String>,
    /// Notes rules wrote that no agent has explained yet.
    #[serde(default)]
    pub unexplained: usize,
    pub kept: usize,
    pub notes_total: usize,
    pub lint_warnings: usize,
    /// Share of wikilinks in CodeWiki-written notes that resolve.
    pub link_rate: f64,
    pub commit: Option<String>,
    /// What the run did about notes whose subject is gone or moved, one
    /// sentence each, for a person or an agent to read. The command line,
    /// the MCP tool and the app all show these lines, so that the three say
    /// the same. Empty for a run that met no such note.
    #[serde(default)]
    pub summary: Vec<String>,
}

/// The first few of `paths`, and how many more there are.
fn some_of(paths: &[String]) -> String {
    const SHOWN: usize = 5;
    let mut text = paths
        .iter()
        .take(SHOWN)
        .map(|path| format!("`{path}`"))
        .collect::<Vec<_>>()
        .join(", ");
    if paths.len() > SHOWN {
        text.push_str(&format!(" and {} more", paths.len() - SHOWN));
    }
    text
}

/// `1 note` or `3 notes`.
fn notes(count: usize) -> String {
    match count {
        1 => "1 note".to_string(),
        n => format!("{n} notes"),
    }
}

impl Report {
    /// The lines of [`Report::summary`], from what the run recorded.
    fn summarize(&self) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        if !self.removed.is_empty() {
            lines.push(format!(
                "Removed {} whose subject is gone: {}.",
                notes(self.removed.len()),
                some_of(&self.removed)
            ));
        }
        if !self.moved.is_empty() {
            let pairs: Vec<String> = self
                .moved
                .iter()
                .map(|m| format!("{} -> {}", m.from, m.to))
                .collect();
            lines.push(format!(
                "Moved {} with its subject, with all that was written in it: {}.",
                notes(self.moved.len()),
                some_of(&pairs)
            ));
        }
        if !self.possible_moves.is_empty() {
            lines.push(format!(
                "{} with written work may describe something that moved, but where to is not clear, so nothing was carried over: {}.",
                notes(self.possible_moves.len()),
                some_of(&self.possible_moves)
            ));
        }
        if !self.retire.is_empty() {
            lines.push(format!(
                "{} written by hand name only files that are gone: {}. None was deleted. They wait in one retire proposal; approve or reject it on the Review page of the CodeWiki app.",
                notes(self.retire.len()),
                some_of(&self.retire)
            ));
        }
        for (why, text) in [
            (
                Gone::Uncommitted,
                "describe something whose deletion is not committed; they stay until it is",
            ),
            (
                Gone::Unsaved,
                "describe something gone and stay, because git does not hold them as they are; commit the wiki and sync again",
            ),
        ] {
            let kept: Vec<String> = self
                .kept_gone
                .iter()
                .filter(|kept| kept.why == why)
                .map(|kept| kept.path.clone())
                .collect();
            if !kept.is_empty() {
                lines.push(format!("{} {text}: {}.", notes(kept.len()), some_of(&kept)));
            }
        }
        if !self.partly_gone.is_empty() {
            lines.push(format!(
                "{} written by hand name files that are gone beside files that exist, and stay as they are: {}.",
                notes(self.partly_gone.len()),
                some_of(&self.partly_gone)
            ));
        }
        if !self.repathed.is_empty() {
            lines.push(format!(
                "In {} written by hand the path of a moved file was replaced by where the file is now: {}.",
                notes(self.repathed.len()),
                some_of(&self.repathed)
            ));
        }
        if !self.unlinked.is_empty() {
            lines.push(format!(
                "In {} a link to a removed note became plain words: {}.",
                notes(self.unlinked.len()),
                some_of(&self.unlinked)
            ));
        }
        if !self.attachments_removed.is_empty() {
            lines.push(format!(
                "Deleted {} that only removed notes showed: {}.",
                match self.attachments_removed.len() {
                    1 => "1 attachment".to_string(),
                    n => format!("{n} attachments"),
                },
                some_of(&self.attachments_removed)
            ));
        }
        if !self.superseded.is_empty() {
            lines.push(format!(
                "A proposal can no longer be approved for {}, because what the note describes went: {}.",
                notes(self.superseded.len()),
                some_of(&self.superseded)
            ));
        }
        lines
    }
}

/// A note a sync left in the vault although its subject is gone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptGone {
    /// Vault-relative path of the note.
    pub path: String,
    pub why: Gone,
}

/// A note that a sync moved because its subject moved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Moved {
    /// Vault-relative path before the run.
    pub from: String,
    /// Vault-relative path now.
    pub to: String,
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
            explained: None,
            behind: Vec::new(),
            cited: Vec::new(),
            gone: None,
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

/// What a note describes, for the kinds of subject that can move: the kind
/// and the name alone, without the place. `type:engine::pricing::Price`
/// gives `("type", "Price")`.
fn movable(id: &str) -> Option<(&str, &str)> {
    let (kind, place) = id.split_once(':')?;
    matches!(kind, "type" | "mod" | "pattern")
        .then(|| (kind, place.rsplit("::").next().unwrap_or(place)))
}

/// Give each note whose subject moved to the planned note of that subject
/// in its new place, so that what was written in it is kept.
///
/// A subject that moves gets a new id, because the id names the place. The
/// old note is then a note of something gone and the new one starts empty.
/// Here the old entry takes the new id and the new path instead. Only a
/// clear match counts: git saw the file move to the one planned note's
/// file, or exactly one new note and one old note share kind and name.
///
/// The files are not touched here. Returns the renames to make, old and new
/// vault path, once the run can no longer fail before it writes.
fn carry_moves(
    vault: &Path,
    meta: &mut VaultMeta,
    notes: &[Planned],
    git: &gone::Git,
    report: &mut Report,
) -> Result<Vec<(String, String)>> {
    let mut fresh: BTreeMap<(&str, &str), Vec<&Planned>> = BTreeMap::new();
    for note in notes.iter().filter(|n| !meta.notes.contains_key(&n.id)) {
        if let Some(key) = movable(&note.id) {
            fresh.entry(key).or_default().push(note);
        }
    }
    if fresh.is_empty() {
        return Ok(Vec::new());
    }
    let planned: HashSet<&str> = notes.iter().map(|n| n.id.as_str()).collect();
    let mut left: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for entry in meta
        .notes
        .values()
        .filter(|e| e.origin == Origin::Generated && !planned.contains(e.id.as_str()))
    {
        if let Some((kind, name)) = movable(&entry.id) {
            left.entry((kind.to_string(), name.to_string()))
                .or_default()
                .push(entry.id.clone());
        }
    }

    let mut renames: Vec<(String, String)> = Vec::new();
    let mut taken: HashSet<&str> = HashSet::new();
    let mut occupied: HashSet<String> =
        meta.notes.values().map(|e| e.path.to_lowercase()).collect();
    for ((kind, name), old_ids) in &left {
        let Some(candidates) = fresh.get(&(kind.as_str(), name.as_str())) else {
            continue;
        };
        for old_id in old_ids {
            let entry = &meta.notes[old_id];
            let free: Vec<&Planned> = candidates
                .iter()
                .copied()
                .filter(|c| !taken.contains(c.id.as_str()))
                .collect();
            let by_git: Vec<&Planned> = free
                .iter()
                .copied()
                .filter(|c| {
                    entry.sources.iter().any(|source| {
                        git.moves()
                            .get(&source.path)
                            .is_some_and(|end| c.sources.contains(end))
                    })
                })
                .collect();
            let target = match (by_git.as_slice(), free.as_slice()) {
                ([one], _) => Some(*one),
                ([], [one]) if old_ids.len() == 1 && candidates.len() == 1 => Some(*one),
                _ => None,
            };
            // The new place must be free: a file already there is not ours
            // to replace.
            let new_path = target.map(planned_path);
            let free_place = new_path.as_ref().is_some_and(|path| {
                *path == entry.path
                    || (!occupied.contains(&path.to_lowercase()) && !vault.join(path).exists())
            });
            let (Some(target), Some(new_path), true) = (target, new_path, free_place) else {
                let written_work = read_optional(&vault.join(&entry.path))?.is_some_and(|text| {
                    entry.explained.is_some() || body_hash(&text) != entry.body_hash
                });
                if written_work {
                    report.possible_moves.push(entry.path.clone());
                }
                continue;
            };
            taken.insert(target.id.as_str());
            let Some(mut entry) = meta.notes.remove(old_id) else {
                continue;
            };
            let old_path = entry.path.clone();
            if entry.explained.is_some() {
                let from = entry
                    .sources
                    .first()
                    .map_or(old_path.as_str(), |source| source.path.as_str());
                entry.behind.retain(|line| !line.starts_with(CARRIED));
                entry.behind.push(format!("{CARRIED}`{from}`"));
            }
            if new_path != old_path {
                renames.push((old_path.clone(), new_path.clone()));
                if entry.illustration.is_some() {
                    let pictures = illustration_path(&new_path);
                    renames.push((illustration_path(&old_path), pictures.clone()));
                    entry.illustration = Some(pictures);
                }
                occupied.insert(new_path.to_lowercase());
                entry.path = new_path;
            }
            entry.id = target.id.clone();
            report.moved.push(Moved {
                from: old_path,
                to: entry.path.clone(),
            });
            meta.notes.insert(entry.id.clone(), entry);
        }
    }
    report.possible_moves.sort();
    Ok(renames)
}

/// Generate or update the vault of the repository at `root`, reading
/// nothing but the repository: a package from outside it is described only
/// by what an earlier run kept in the vault.
pub fn generate(
    root: &Path,
    facts: &RepoFacts,
    metrics: &[(String, NodeMetrics)],
    commit: Option<String>,
) -> Result<Report> {
    generate_with(root, facts, metrics, commit, &NoPackageInfo)
}

/// [`generate`], with `packages` to say what a package from outside the
/// repository is.
pub fn generate_with(
    root: &Path,
    facts: &RepoFacts,
    metrics: &[(String, NodeMetrics)],
    commit: Option<String>,
    packages: &dyn PackageInfo,
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

    // A note an agent explained keeps its place while its subject exists.
    let pinned: HashSet<String> = meta
        .notes
        .values()
        .filter(|e| e.origin == Origin::Generated && e.explained.is_some())
        .map(|e| e.id.clone())
        .collect();

    let mut repo = Repo::at(root, facts, metrics);
    let Plan {
        mut notes,
        owner,
        files,
    } = plan(&repo, &covered, &pinned);
    let git = gone::Git::new(root);
    let renames = carry_moves(&vault, &mut meta, &notes, &git, &mut report)?;
    let integrations: Vec<&str> = notes
        .iter()
        .filter_map(|note| match &note.subject {
            Subject::Integration(package) => Some(package.as_str()),
            _ => None,
        })
        .collect();
    repo.packages.descriptions = manifests::describe(&repo, &vault, &integrations, packages)?;

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

    // Notes their owner wrote are about the paths they name. One that
    // names only paths that are gone describes nothing any more. It is
    // never deleted by a run: the run proposes it, with the others, and
    // its owner decides. This comes before the drafts, because the home
    // note lists the folders of the notes written by hand that stay.
    let in_plan: HashSet<&str> = notes.iter().map(|n| n.id.as_str()).collect();
    let by_hand: Vec<String> = meta
        .notes
        .values()
        .filter(|e| e.origin == Origin::Adopted && !in_plan.contains(e.id.as_str()))
        .map(|e| e.id.clone())
        .collect();
    let mut retire: Vec<proposal::Retired> = Vec::new();
    for id in by_hand {
        let Some(entry) = meta.notes.get_mut(&id) else {
            continue;
        };
        let full = vault.join(&entry.path);
        let Some(mut text) = read_optional(&full)? else {
            continue;
        };
        let mut cited = gone::cited(&text, &git);
        // A file that moved: the note follows it, when git holds the note
        // as it is, so that the change can be taken back.
        if !cited.moved.is_empty() && git.holds(entry) {
            text = gone::repath(&text, &cited.moved);
            write_note(&full, &text)?;
            entry.body_hash = body_hash(&text);
            report.repathed.push(entry.path.clone());
            cited = gone::cited(&text, &git);
        }
        entry.cited = cited.all();
        entry.gone = match gone::of_adopted(entry, &cited, &git) {
            gone::Hand::Unjudged | gone::Hand::Alive => None,
            gone::Hand::PartlyGone => {
                report.partly_gone.push(entry.path.clone());
                None
            }
            gone::Hand::Gone(why) => {
                match why {
                    Gone::Pending => {
                        retire.push(proposal::Retired {
                            note_id: entry.id.clone(),
                            path: entry.path.clone(),
                            base_hash: body_hash(&text),
                            cited: cited.dead.clone(),
                        });
                        report.retire.push(entry.path.clone());
                    }
                    // Decided once; no news.
                    Gone::History => {}
                    Gone::Uncommitted | Gone::Unsaved => report.kept_gone.push(KeptGone {
                        path: entry.path.clone(),
                        why,
                    }),
                }
                Some(why)
            }
        };
    }
    report.retire.sort();
    report.partly_gone.sort();
    report.repathed.sort();
    proposal::create_retire(&vault, retire)?;

    // The folders of the notes written by hand, for the home note. A note
    // that waits to be retired is on its way out and is not counted.
    let mut hand_zones: BTreeMap<String, HandZone> = BTreeMap::new();
    for entry in meta.notes.values().filter(|e| {
        e.origin == Origin::Adopted
            && !in_plan.contains(e.id.as_str())
            && e.gone != Some(Gone::Pending)
    }) {
        let Some((folder, rest)) = entry.path.split_once('/') else {
            continue;
        };
        let zone = hand_zones.entry(folder.to_string()).or_default();
        if rest == "_index.md" {
            zone.index = true;
        } else if !matches!(entry.kind, NoteKind::Home | NoteKind::Index) {
            let file = rest.rsplit('/').next().unwrap_or(rest);
            zone.notes.push(file.trim_end_matches(".md").to_string());
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
    let plan = Plan {
        notes,
        owner,
        files,
    };
    let mut ctx = RenderCtx::new(&repo, &plan, &registry, &repo_name);
    ctx.hand_zones = hand_zones;
    let file_hashes: HashMap<&str, &str> = facts
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.hash.as_str()))
        .collect();

    // Every draft must pass the linter: it is what the rules write, and what
    // an agent's explanation is later woven into.
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

    // Notes CodeWiki wrote whose subject no longer exists go at the end of
    // this run. They are known now, since a link to one of them is dead
    // already. One that holds written work, an agent's explanation or its
    // owner's edits, goes only when git can bring it back and the deletion
    // of its subject is committed; until then it stays, marked.
    let planned_ids: HashSet<&str> = plan.notes.iter().map(|n| n.id.as_str()).collect();
    // The drafts are sound, so the run will write: the notes that follow
    // their subject move now, before any of them is read.
    for (from, to) in &renames {
        let (from, to) = (vault.join(from), vault.join(to));
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        match std::fs::rename(&from, &to) {
            Ok(()) => {}
            // A picture file the manifest names may be gone already.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io(&from, e)),
        }
    }
    let mut leaving: HashSet<String> = HashSet::new();
    let mut staying: Vec<(String, Gone)> = Vec::new();
    for entry in meta
        .notes
        .values()
        .filter(|e| e.origin == Origin::Generated && !planned_ids.contains(e.id.as_str()))
    {
        let written_work = read_optional(&vault.join(&entry.path))?
            .is_some_and(|text| entry.explained.is_some() || body_hash(&text) != entry.body_hash);
        if !written_work {
            leaving.insert(entry.id.clone());
            continue;
        }
        match gone::of_generated(entry, &git) {
            Verdict::Remove => {
                leaving.insert(entry.id.clone());
            }
            Verdict::Keep(why) => staying.push((entry.id.clone(), why)),
        }
    }
    for (id, why) in staying {
        if let Some(entry) = meta.notes.get_mut(&id) {
            entry.gone = Some(why);
            report.kept_gone.push(KeptGone {
                path: entry.path.clone(),
                why,
            });
        }
    }
    report.kept_gone.sort_by(|a, b| a.path.cmp(&b.path));
    let links = Links {
        registry: &registry,
        leaving: &leaving,
        dead: Default::default(),
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
        // The hash covers what the note is written from, never the note.
        let input_hash = sha256_hex(format!("v{TEMPLATE_VERSION}\n{draft}"));
        let fresh_entry = |text: &str| NoteEntry {
            id: note.id.clone(),
            path: path.clone(),
            kind: note.kind,
            origin: Origin::Generated,
            sources: sources.clone(),
            body_hash: body_hash(text),
            input_hash: input_hash.clone(),
            rejected_inputs: Vec::new(),
            commit: commit.clone(),
            model: None,
            illustration: None,
            explained: None,
            behind: Vec::new(),
            cited: Vec::new(),
            gone: None,
        };

        let Some(entry) = meta.notes.get_mut(&note.id) else {
            write_note(&full, draft)?;
            report.written.push(path.clone());
            report.unexplained += usize::from(note.kind.explainable());
            meta.notes.insert(note.id.clone(), fresh_entry(draft));
            continue;
        };
        // It has a place in the plan, so what it describes exists.
        entry.gone = None;
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
        // Reasons the explanation is behind that this run itself gives.
        let mut moved: Vec<String> = Vec::new();
        match lifecycle::action(entry, state, &input_hash) {
            Action::Keep => {
                // No fact changed, but a cited file may have, in a way no
                // fact shows. Record the files as this run read them, or a
                // brief would ask for a sync that changes nothing.
                entry.sources = sources.clone();
                report.kept += 1;
                // A link of the agent's may have died while the facts
                // stayed: its note left in an earlier run. Only a note
                // nobody edited is mended here; its owner's is theirs.
                let untouched = matches!(state, NoteState::Generated | NoteState::Stale);
                if let (true, Some(current)) = (entry.explained.is_some() && untouched, &disk) {
                    let woven = weave_kept(entry, current, current, note.kind, &links, false);
                    let dead = links.take_dead();
                    if !dead.is_empty() && woven.text != *current {
                        write_note(&full, &woven.text)?;
                        entry.body_hash = body_hash(&woven.text);
                        report.unlinked.push(path.clone());
                        for name in dead {
                            moved.push(format!("{}`{name}`", gone::UNLINKED));
                        }
                    }
                }
            }
            Action::Merge => {
                // The facts are the fresh draft's; the explanation is the
                // agent's, as it stands in the note.
                let current = disk.clone().unwrap_or_default();
                let woven = weave_kept(entry, &current, draft, note.kind, &links, false);
                write_note(&full, &woven.text)?;
                report.merged.push(path.clone());
                // The agent's own words changed: a link became plain.
                for name in links.take_dead() {
                    moved.push(format!("{}`{name}`", gone::UNLINKED));
                }
                for heading in &woven.folded {
                    moved.push(format!(
                        "{MOVED}`{heading}` is no section of this kind of note any more; its text was moved"
                    ));
                }
                entry.kind = note.kind;
                entry.sources = sources.clone();
                entry.body_hash = body_hash(&woven.text);
                entry.input_hash = input_hash.clone();
                entry.commit = commit.clone();
                if let Some(explained) = entry.explained.as_mut() {
                    explained.sections = woven.written;
                }
                keep_placed_pictures(&vault, entry, &woven.text)?;
            }
            Action::Regenerate => {
                write_note(&full, draft)?;
                report.written.push(path.clone());
                let rejected = std::mem::take(&mut entry.rejected_inputs);
                *entry = fresh_entry(draft);
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
                    // The proposal is about the facts. What an agent
                    // wrote stays in what is proposed, and so does a
                    // section the owner added that rules do not write.
                    let proposed = weave_kept(entry, &current, draft, note.kind, &links, true).text;
                    // Its owner sees the change in the proposal.
                    links.take_dead();
                    proposal::create(
                        &vault,
                        &note.id,
                        path,
                        &current,
                        &proposed,
                        &input_hash,
                        "cited source code changed",
                    )?;
                }
                report.proposed.push(path.clone());
            }
        }

        let Some(explained) = &entry.explained else {
            entry.behind.clear();
            // A note its owner changed is not in the agents' queue: what
            // an agent wrote for it would only be a proposal.
            let untouched = state != NoteState::HumanEdited && state != NoteState::HumanEditedStale;
            report.unexplained += usize::from(note.kind.explainable() && untouched);
            continue;
        };
        // The explanation was written from files. One that reads otherwise
        // now may have made it wrong; only an agent can tell.
        let mut behind: Vec<String> = explained
            .sources
            .iter()
            .filter_map(|source| {
                let now = file_hashes
                    .get(source.path.as_str())
                    .map(|hash| hash.to_string())
                    .or_else(|| file_hash(root, &source.path));
                match now {
                    Some(hash) if hash == source.hash => None,
                    Some(_) => Some(format!("{} changed", source.path)),
                    None => Some(format!("{} is gone", source.path)),
                }
            })
            .collect();
        // A section that moved, a subject that moved and a link that became
        // plain words stay a reason until an agent writes again.
        behind.extend(
            entry
                .behind
                .iter()
                .filter(|l| {
                    l.starts_with(MOVED) || l.starts_with(CARRIED) || l.starts_with(gone::UNLINKED)
                })
                .cloned(),
        );
        behind.extend(moved);
        if !behind.is_empty() {
            report.behind.push(path.clone());
        }
        entry.behind = behind;
    }
    // A picture explains what a note says, and rules cannot tell what a
    // thing is for: they draw none. Only an agent that explained a note
    // gives it pictures, in `<Note>.openui` beside it. A picture file of a
    // note no agent explained is one that rules drew before; it goes.
    for entry in meta.notes.values_mut().filter(|e| e.explained.is_none()) {
        entry.illustration = None;
        let full = vault.join(illustration_path(&entry.path));
        match std::fs::remove_file(&full) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io(&full, e)),
        }
    }

    // The notes whose subject no longer exists.
    let mut leaving: Vec<String> = leaving.into_iter().collect();
    leaving.sort();
    let mut left: Vec<gone::Left> = Vec::new();
    for id in leaving {
        let Some(entry) = meta.notes.remove(&id) else {
            continue;
        };
        let full = vault.join(&entry.path);
        let text = read_optional(&full)?.unwrap_or_default();
        match std::fs::remove_file(&full) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io(&full, e)),
        }
        // Pictures have no meaning without their note.
        let _ = std::fs::remove_file(vault.join(illustration_path(&entry.path)));
        left.push(gone::Left {
            path: entry.path.clone(),
            text,
        });
        report.removed.push(entry.path);
    }
    // A proposal that waits for a note whose subject went would write
    // facts about code that is not there. Nobody may approve it any more.
    for waiting in proposal::list(&vault)? {
        if waiting.status != proposal::ProposalStatus::Pending
            || waiting.kind == proposal::ProposalKind::Retire
        {
            continue;
        }
        let outdated = match meta.notes.get(&waiting.note_id) {
            Some(entry) => {
                entry.gone.is_some()
                    // A sync proposes from a draft, and there is none.
                    || (waiting.kind == proposal::ProposalKind::Sync
                        && !planned_ids.contains(entry.id.as_str()))
            }
            None => left.iter().any(|note| note.path == waiting.note_path),
        };
        if outdated {
            proposal::supersede(&vault, &waiting.id)?;
            report.superseded.push(waiting.note_path);
        }
    }
    report.superseded.sort();

    // What the notes that stay still say of the ones that left, and what
    // only those showed.
    let cleanup = gone::after_removal(&vault, &mut meta, &git, &left)?;
    report.unlinked.extend(cleanup.unlinked);
    report.unlinked.sort();
    report.unlinked.dedup();
    report.attachments_removed = cleanup.attachments;
    gone::remove_empty_folders(&vault)?;

    write_readme(&vault, &repo_name)?;
    meta.manifest.template_version = TEMPLATE_VERSION;
    meta.manifest.last_indexed_commit = commit;
    meta.save(&vault)?;

    let final_registry = meta.registry()?;
    let own: Vec<(String, String)> = meta
        .notes
        .values()
        // A note kept after its subject went is frozen, and may link what
        // left with the subject: the rate is that of the live wiki.
        .filter(|e| e.origin == Origin::Generated && e.gone.is_none())
        .filter_map(|e| Some((e.path.clone(), read_optional(&vault.join(&e.path)).ok()??)))
        .collect();
    report.link_rate = link_resolution_rate(
        own.iter().map(|(p, t)| (p.as_str(), t.as_str())),
        &final_registry,
    );
    report.notes_total = meta.live_notes();
    report.summary = report.summarize();
    Ok(report)
}

/// How a line of [`NoteEntry::behind`] starts that says a section of the
/// agent's lost its place.
const MOVED: &str = "the section ";

/// How a line of [`NoteEntry::behind`] starts that says the note followed
/// its subject to another place in the code.
const CARRIED: &str = "its subject moved from ";

/// The hash of the repository file `path` as it is now, made as a scan
/// makes it; nothing for a file that is gone or leaves the repository.
pub(crate) fn file_hash(root: &Path, path: &str) -> Option<String> {
    let inside = !path.is_empty()
        && !path.contains(':')
        && !path.starts_with(['/', '\\'])
        && path.split(['/', '\\']).all(|part| part != "..");
    if !inside {
        return None;
    }
    std::fs::read(root.join(path)).ok().map(sha256_hex)
}

/// Which links of a note lead to a note once this run is over.
struct Links<'a> {
    registry: &'a Registry,
    /// Ids of the notes this run removes.
    leaving: &'a HashSet<String>,
    /// The targets found dead since they were last taken, each once.
    dead: std::cell::RefCell<Vec<String>>,
}

impl Links<'_> {
    fn alive(&self, target: &str, from_path: &str) -> bool {
        let alive = self
            .registry
            .resolve(target, from_path)
            .is_some_and(|entry| !self.leaving.contains(&entry.id));
        if !alive && !self.dead.borrow().iter().any(|dead| dead == target) {
            self.dead.borrow_mut().push(target.to_string());
        }
        alive
    }

    /// The dead targets met since the last call, in the order met.
    fn take_dead(&self) -> Vec<String> {
        std::mem::take(&mut self.dead.borrow_mut())
    }
}

/// The fresh `draft` of a note with the sections an agent wrote, as they
/// stand in `current`, the note on disk. A link of the agent's to a note
/// that is gone becomes plain words.
///
/// With `owners_too`, for a note its owner edited: a section that rules do
/// not write at all, such as `Gotchas`, is kept as well. It can only be the
/// owner's, and the draft has nothing to put in its place.
fn weave_kept(
    entry: &NoteEntry,
    current: &str,
    draft: &str,
    kind: NoteKind,
    links: &Links,
    owners_too: bool,
) -> cw_vault::Woven {
    let mut headings: Vec<String> = entry
        .explained
        .as_ref()
        .map(|explained| explained.sections.clone())
        .unwrap_or_default();
    if owners_too {
        let (_, drafted) = cw_vault::note::split(draft);
        for heading in kind.agent_sections() {
            let by_rules = drafted.iter().any(|s| s.heading == *heading);
            if !by_rules && !headings.iter().any(|h| h == heading) {
                headings.push(heading.to_string());
            }
        }
    }
    let mut agent = cw_vault::extract(current, kind, &headings);
    for section in &mut agent {
        section.body =
            cw_vault::unlink_dead(&section.body, |target| links.alive(target, &entry.path));
    }
    cw_vault::weave(draft, &agent, kind)
}

/// Keep the pictures of a note whose section the note `text` still has. A
/// picture stands inside a section and has no place without it.
fn keep_placed_pictures(vault: &Path, entry: &mut NoteEntry, text: &str) -> Result<()> {
    let relative = illustration_path(&entry.path);
    let full = vault.join(&relative);
    let Some(file) = read_optional(&full)? else {
        entry.illustration = None;
        return Ok(());
    };
    let (_, sections) = cw_vault::note::split(text);
    let mut pictures = cw_illustrate::parse_pictures(&file);
    let before = pictures.len();
    pictures.retain(|picture| sections.iter().any(|s| s.heading == picture.section));
    if pictures.is_empty() {
        std::fs::remove_file(&full).map_err(|e| Error::io(&full, e))?;
        entry.illustration = None;
    } else {
        if pictures.len() != before {
            write_note(&full, &cw_illustrate::serialize_pictures(&pictures))?;
        }
        entry.illustration = Some(relative);
    }
    Ok(())
}

/// Vault path of the file that holds the pictures of the note at `note_path`.
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
