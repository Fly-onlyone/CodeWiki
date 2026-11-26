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

use crate::manifests::{self, NoPackageInfo, PackageInfo};
use crate::plan::{Plan, Planned, Repo, Subject, plan};
use crate::render::RenderCtx;

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
    /// Notes rules wrote that no agent has explained yet.
    #[serde(default)]
    pub unexplained: usize,
    pub kept: usize,
    pub notes_total: usize,
    pub lint_warnings: usize,
    /// Share of wikilinks in CodeWiki-written notes that resolve.
    pub link_rate: f64,
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
            explained: None,
            behind: Vec::new(),
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

    let mut repo = Repo::at(root, facts, metrics);
    let Plan { mut notes, owner } = plan(&repo, &covered);
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
    // this run, unless their owner changed them. They are known now, since
    // a link to one of them is dead already.
    let planned_ids: HashSet<&str> = plan.notes.iter().map(|n| n.id.as_str()).collect();
    let mut leaving: HashSet<String> = HashSet::new();
    for entry in meta
        .notes
        .values()
        .filter(|e| e.origin == Origin::Generated && !planned_ids.contains(e.id.as_str()))
    {
        let untouched = read_optional(&vault.join(&entry.path))?
            .is_none_or(|text| body_hash(&text) == entry.body_hash);
        if untouched {
            leaving.insert(entry.id.clone());
        }
    }
    let links = Links {
        registry: &registry,
        leaving: &leaving,
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
        };

        let Some(entry) = meta.notes.get_mut(&note.id) else {
            write_note(&full, draft)?;
            report.written.push(path.clone());
            report.unexplained += usize::from(note.kind.explainable());
            meta.notes.insert(note.id.clone(), fresh_entry(draft));
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
        // Reasons the explanation is behind that this run itself gives.
        let mut moved: Vec<String> = Vec::new();
        match lifecycle::action(entry, state, &input_hash) {
            Action::Keep => {
                // No fact changed, but a cited file may have, in a way no
                // fact shows. Record the files as this run read them, or a
                // brief would ask for a sync that changes nothing.
                entry.sources = sources.clone();
                report.kept += 1;
            }
            Action::Merge => {
                // The facts are the fresh draft's; the explanation is the
                // agent's, as it stands in the note.
                let current = disk.clone().unwrap_or_default();
                let woven = weave_kept(entry, &current, draft, note.kind, &links, false);
                write_note(&full, &woven.text)?;
                report.merged.push(path.clone());
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
        // A section that moved stays a reason until an agent writes again.
        behind.extend(
            entry
                .behind
                .iter()
                .filter(|l| l.starts_with(MOVED))
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
    for id in leaving {
        let Some(entry) = meta.notes.remove(&id) else {
            continue;
        };
        let full = vault.join(&entry.path);
        match std::fs::remove_file(&full) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io(&full, e)),
        }
        // Pictures have no meaning without their note.
        let _ = std::fs::remove_file(vault.join(illustration_path(&entry.path)));
        report.removed.push(entry.path);
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

/// How a line of [`NoteEntry::behind`] starts that says a section of the
/// agent's lost its place.
const MOVED: &str = "the section ";

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
}

impl Links<'_> {
    fn alive(&self, target: &str, from_path: &str) -> bool {
        self.registry
            .resolve(target, from_path)
            .is_some_and(|entry| !self.leaving.contains(&entry.id))
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
