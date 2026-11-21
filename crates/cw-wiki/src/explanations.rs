//! Explanations: what a coding agent writes into a note, and how.
//!
//! Rules write every note from the code. They state facts and cannot say
//! what a thing is for. A coding agent can: it reads the code. This module
//! is its way in, and the only one:
//!
//! - [`queue`] lists the notes that wait for an explanation.
//! - [`brief`] gives the agent one note: its text, which sections are the
//!   agent's, the files to read, and the limits.
//! - [`submit`] takes the agent's sections and pictures. It checks them and
//!   weaves them into the note. The agent never sends a whole note, so it
//!   cannot change a fact.
//!
//! A note its owner edited is never written: what the agent sends for it
//! is filed as a proposal for the owner to review.

use std::path::Path;

use cw_core::{Error, Result, VAULT_DIR};
use cw_illustrate::Picture;
use cw_vault::lint::{Mode, Severity, lint};
use cw_vault::proposal::{self, Offer, ProposalKind};
use cw_vault::{Explained, NoteEntry, NoteKind, Origin, Section, SourceRef, VaultMeta, body_hash};
use serde::{Deserialize, Serialize};

use crate::generate::{file_hash, illustration_path};
use crate::query::{find_entry, title_of};

/// Notes a queue lists when the caller names no number.
pub const DEFAULT_LIMIT: usize = 5;
/// Notes a queue lists at most. An agent writes a note at a time, and each
/// costs its user's allowance.
pub const MAX_LIMIT: usize = 20;
/// Words an agent's text for one section holds at most.
pub const MAX_SECTION_WORDS: usize = 300;
/// Files beside the note's sources that a submission may name as read.
pub const MAX_READ: usize = 40;

/// How many notes wait for an agent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counts {
    /// Explained, and a file the explanation was written from changed.
    pub behind: usize,
    /// Written by rules; no agent has explained them yet.
    pub unexplained: usize,
}

impl Counts {
    pub fn total(self) -> usize {
        self.behind + self.unexplained
    }
}

/// Why a note waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Why {
    /// Its explanation may be behind the code. These come first: a wrong
    /// explanation is worse than none.
    Behind,
    /// No agent has explained it.
    Unexplained,
}

/// One note of the queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Waiting {
    pub path: String,
    pub title: String,
    pub kind: NoteKind,
    pub why: Why,
    /// For a note that is behind: what changed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub behind: Vec<String>,
    /// The files the note is about.
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Queue {
    /// The first notes to write, at most the limit asked for.
    pub notes: Vec<Waiting>,
    /// How many notes fit what was asked for, listed or not.
    pub matching: usize,
    /// How many notes wait in the whole vault.
    pub waiting: Counts,
}

/// Which waiting notes a queue lists.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scope {
    /// Only notes in this folder of the vault, such as `01-Core/engine`.
    pub folder: Option<String>,
    /// Only notes about one of these repository files.
    pub files: Vec<String>,
    /// How many to list; [`DEFAULT_LIMIT`] when left out.
    pub limit: Option<usize>,
}

/// A repository or vault path as the manifest writes it.
fn tidy(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_matches('/')
        .to_string()
}

/// Whether the note is as CodeWiki last wrote it: nobody edited it.
fn untouched(vault: &Path, entry: &NoteEntry) -> bool {
    entry.origin == Origin::Generated
        && std::fs::read_to_string(vault.join(&entry.path))
            .is_ok_and(|text| body_hash(&text) == entry.body_hash)
}

/// Why the note waits for an agent, if it does. A note its owner edited
/// does not: what an agent wrote for it would only be a proposal, again at
/// every run.
fn waits(vault: &Path, entry: &NoteEntry) -> Option<Why> {
    if !entry.kind.explainable() {
        return None;
    }
    let why = match &entry.explained {
        Some(_) if entry.behind.is_empty() => return None,
        Some(_) => Why::Behind,
        None => Why::Unexplained,
    };
    untouched(vault, entry).then_some(why)
}

fn count(waiting: &[(Why, &NoteEntry)]) -> Counts {
    let behind = waiting
        .iter()
        .filter(|(why, _)| *why == Why::Behind)
        .count();
    Counts {
        behind,
        unexplained: waiting.len() - behind,
    }
}

/// Every waiting note: those behind first, then by path.
fn all_waiting<'a>(vault: &Path, meta: &'a VaultMeta) -> Vec<(Why, &'a NoteEntry)> {
    let mut waiting: Vec<(Why, &NoteEntry)> = meta
        .notes
        .values()
        .filter_map(|entry| Some((waits(vault, entry)?, entry)))
        .collect();
    waiting.sort_by(|a, b| (a.0, &a.1.path).cmp(&(b.0, &b.1.path)));
    waiting
}

/// How many notes of the repository at `root` wait for an agent.
pub fn waiting_counts(root: &Path) -> Result<Counts> {
    let vault = root.join(VAULT_DIR);
    let meta = VaultMeta::load(&vault)?;
    Ok(count(&all_waiting(&vault, &meta)))
}

/// The notes that wait for an agent, as far as `scope` asks.
pub fn queue(root: &Path, scope: &Scope) -> Result<Queue> {
    let vault = root.join(VAULT_DIR);
    let meta = VaultMeta::load(&vault)?;
    let waiting = all_waiting(&vault, &meta);
    let counts = count(&waiting);
    let folder = scope.folder.as_deref().map(tidy).filter(|f| !f.is_empty());
    let files: Vec<String> = scope.files.iter().map(|f| tidy(f)).collect();
    let fits = |entry: &NoteEntry| {
        let in_folder = folder.as_deref().is_none_or(|folder| {
            entry
                .path
                .to_lowercase()
                .starts_with(&format!("{}/", folder.to_lowercase()))
        });
        let about = files.is_empty()
            || entry
                .sources
                .iter()
                .chain(entry.explained.iter().flat_map(|e| e.sources.iter()))
                .any(|source| files.contains(&source.path));
        in_folder && about
    };
    let matching: Vec<&(Why, &NoteEntry)> =
        waiting.iter().filter(|(_, entry)| fits(entry)).collect();
    let limit = scope.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    Ok(Queue {
        notes: matching
            .iter()
            .take(limit)
            .map(|(why, entry)| Waiting {
                path: entry.path.clone(),
                title: title_of(entry),
                kind: entry.kind,
                why: *why,
                behind: entry.behind.clone(),
                sources: entry.sources.iter().map(|s| s.path.clone()).collect(),
            })
            .collect(),
        matching: matching.len(),
        waiting: counts,
    })
}

/// One of the agent's sections of a note, in a [`Brief`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Yours {
    pub heading: String,
    /// What the section holds now; empty when the note has none. In a
    /// section whose code blocks are facts, the prose alone.
    pub text: String,
    /// `agent`, `rules`, or `nobody` for a section the note lacks.
    pub written_by: String,
    /// The section's code blocks are facts that rules write: send prose,
    /// and no code block.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub prose_only: bool,
}

/// A file a note is about, in a [`Brief`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BriefSource {
    pub path: String,
    /// It reads otherwise than when the note was last explained.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub changed: bool,
}

/// What a submission must stay within.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    /// Words of the text for one section, at most.
    pub section_words: usize,
    /// The limits of pictures, in words.
    pub pictures: String,
    /// The components a picture is made of, one signature a line.
    pub components: String,
}

/// All an agent needs to explain one note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Brief {
    pub path: String,
    pub title: String,
    pub kind: NoteKind,
    /// The note as it is.
    pub text: String,
    /// Names this version of the note. A submission sends it back, so that
    /// one written against an older version is not woven into a newer.
    pub base: String,
    /// The sections an agent writes in a note of this kind, in their order.
    pub yours: Vec<Yours>,
    /// The sections rules write. They are facts; a submission cannot
    /// change them.
    pub rules: Vec<String>,
    /// The files to read before writing.
    pub sources: Vec<BriefSource>,
    /// Why the explanation may be behind, when it is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub behind: Vec<String>,
    /// The note's pictures now.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pictures: Vec<Picture>,
    /// Who explained the note, if anyone did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explained_by: Option<String>,
    /// Its owner edited the note: a submission becomes a proposal for the
    /// owner to review, and the note stays as it is.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub edited_by_hand: bool,
    /// The code under the note changed since the last sync, so its facts
    /// are old: sync before writing, or the submission is refused.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sync_first: bool,
    pub limits: Limits,
}

fn note_text(vault: &Path, entry: &NoteEntry) -> Result<Option<String>> {
    let full = vault.join(&entry.path);
    match std::fs::read_to_string(&full) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::io(full, e)),
    }
}

/// The note's sources that read otherwise than at the last sync. A file
/// too large to hash has no hash on record and never counts.
fn changed_since_sync(root: &Path, entry: &NoteEntry) -> Vec<String> {
    entry
        .sources
        .iter()
        .filter(|s| !s.hash.is_empty() && file_hash(root, &s.path).as_deref() != Some(&s.hash))
        .map(|s| s.path.clone())
        .collect()
}

fn pictures_of(vault: &Path, entry: &NoteEntry) -> Vec<Picture> {
    std::fs::read_to_string(vault.join(illustration_path(&entry.path)))
        .map(|file| cw_illustrate::parse_pictures(&file))
        .unwrap_or_default()
}

fn not_explainable(entry: &NoteEntry) -> Error {
    Error::invalid(format!(
        "'{}' is a note no agent explains: only notes about a module, a type, an integration, a flow, a pattern or operations are",
        entry.path
    ))
}

/// What an agent needs to explain the note at vault path or name
/// `path_or_name`.
pub fn brief(root: &Path, path_or_name: &str) -> Result<Brief> {
    let vault = root.join(VAULT_DIR);
    let meta = VaultMeta::load(&vault)?;
    let entry = find_entry(&meta, path_or_name)?;
    if !entry.kind.explainable() {
        return Err(not_explainable(entry));
    }
    let text = note_text(&vault, entry)?
        .ok_or_else(|| Error::not_found(format!("the file of note '{}'", entry.path)))?;
    let (_, sections) = cw_vault::note::split(&text);
    let by_agent: &[String] = entry
        .explained
        .as_ref()
        .map(|e| e.sections.as_slice())
        .unwrap_or(&[]);
    let yours = entry
        .kind
        .agent_sections()
        .iter()
        .map(|heading| {
            let prose_only = entry.kind.fact_fences(heading);
            let body = sections
                .iter()
                .find(|s| s.heading == *heading)
                .map(|s| s.body.as_str());
            // Read through `extract`, which leaves the rules' code blocks
            // out of a section that holds both.
            let own = cw_vault::extract(&text, entry.kind, &[heading.to_string()]);
            Yours {
                heading: heading.to_string(),
                text: own.first().map(|s| s.body.clone()).unwrap_or_default(),
                written_by: match body {
                    None => "nobody",
                    Some(_) if by_agent.iter().any(|h| h == heading) => "agent",
                    Some(_) => "rules",
                }
                .to_string(),
                prose_only,
            }
        })
        .collect();
    let explained_from = entry.explained.as_ref().map(|e| e.sources.as_slice());
    let mut sources: Vec<BriefSource> = Vec::new();
    for source in entry
        .sources
        .iter()
        .chain(explained_from.unwrap_or(&[]).iter())
    {
        if sources.iter().any(|s| s.path == source.path) {
            continue;
        }
        // Against the hash the explanation was written from, if there is
        // one; a note never explained has nothing that could be behind.
        let then = explained_from
            .and_then(|all| all.iter().find(|s| s.path == source.path))
            .map(|s| s.hash.as_str());
        sources.push(BriefSource {
            path: source.path.clone(),
            changed: then.is_some_and(|then| {
                !then.is_empty() && file_hash(root, &source.path).as_deref() != Some(then)
            }),
        });
    }
    Ok(Brief {
        path: entry.path.clone(),
        title: title_of(entry),
        kind: entry.kind,
        base: body_hash(&text),
        yours,
        rules: entry
            .kind
            .sections()
            .iter()
            .filter(|h| !entry.kind.agent_sections().contains(h))
            .map(|h| h.to_string())
            .collect(),
        sources,
        behind: entry.behind.clone(),
        pictures: pictures_of(&vault, entry),
        explained_by: entry.explained.as_ref().and(entry.model.clone()),
        edited_by_hand: entry.origin == Origin::Adopted || body_hash(&text) != entry.body_hash,
        sync_first: !changed_since_sync(root, entry).is_empty(),
        limits: Limits {
            section_words: MAX_SECTION_WORDS,
            pictures: cw_illustrate::limits(),
            components: cw_illustrate::library().signatures(),
        },
        text,
    })
}

/// The text an agent wrote for one section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Written {
    pub heading: String,
    pub text: String,
}

/// What an agent sends for one note.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Submission {
    /// The `base` of the brief the agent wrote from.
    pub base: String,
    /// The sections it wrote. One it leaves out stays as it is.
    #[serde(default)]
    pub sections: Vec<Written>,
    /// At most one picture for a section. A section sent without one loses
    /// the picture it had, which explained the old text; the pictures of
    /// sections that are not sent stay.
    #[serde(default)]
    pub pictures: Vec<Picture>,
    /// Repository files it read beside the note's sources. A later change
    /// to one of them marks the explanation behind.
    #[serde(default)]
    pub read: Vec<String>,
    /// The agent, as it names itself.
    #[serde(default)]
    pub model: Option<String>,
}

/// What became of a submission. Each is an answer, not an error: an agent
/// acts on every one of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
    /// The note is written.
    Saved {
        path: String,
        /// The sections of the note that hold an agent's text now.
        sections: Vec<String>,
        /// How many pictures the note has now.
        pictures: usize,
        /// How many notes still wait.
        waiting: Counts,
    },
    /// Nothing was written. Fix what each line names and send again.
    Issues { issues: Vec<String> },
    /// The note changed since the brief. Read the brief again.
    Changed { message: String },
    /// The code under the note changed since the last sync. Sync, then
    /// read the brief again.
    SyncFirst { message: String },
    /// Its owner edited the note, so it was not written: the submission
    /// waits in the review queue.
    Proposed {
        path: String,
        proposal: String,
        message: String,
    },
}

fn words(text: &str) -> usize {
    text.split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .count()
}

/// What is wrong with the text for one section, before it is woven in.
fn review_section(kind: NoteKind, written: &Written, issues: &mut Vec<String>) {
    let heading = &written.heading;
    if written.text.trim().is_empty() {
        issues.push(format!(
            "`{heading}`: the text is empty; leave a section out to keep it as it is"
        ));
        return;
    }
    let count = words(&written.text);
    if count > MAX_SECTION_WORDS {
        issues.push(format!(
            "`{heading}`: {count} words; at most {MAX_SECTION_WORDS}"
        ));
    }
    let mut in_fence = false;
    let mut fenced = false;
    for line in written.text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            fenced = true;
        } else if !in_fence && (line.starts_with("# ") || line.starts_with("## ")) {
            // It would end the section, or start one of the rules'.
            issues.push(format!(
                "`{heading}`: a section's text holds no `#` or `##` heading; use `###` or plain text"
            ));
            break;
        }
    }
    if in_fence {
        issues.push(format!("`{heading}`: a code block is not closed"));
    }
    if fenced && kind.fact_fences(heading) {
        issues.push(format!(
            "`{heading}`: the code blocks of this section are facts that CodeWiki writes; send prose only"
        ));
    }
}

/// The name an agent gave itself, fit to be stored and shown.
fn agent_name(model: Option<&str>) -> String {
    let name: String = model
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_control())
        .take(80)
        .collect();
    let name = name.trim();
    if name.is_empty() {
        "a coding agent".to_string()
    } else {
        name.to_string()
    }
}

/// Take what an agent wrote for the note at vault path or name
/// `path_or_name`: check it, weave it into the note, save its pictures and
/// record what it was written from. See [`Outcome`] for what can come of
/// it. The search index is the caller's to refresh after `Saved`.
pub fn submit(root: &Path, path_or_name: &str, submission: &Submission) -> Result<Outcome> {
    let vault = root.join(VAULT_DIR);
    let mut meta = VaultMeta::load(&vault)?;
    let entry = find_entry(&meta, path_or_name)?.clone();
    let kind = entry.kind;
    if !kind.explainable() {
        return Err(not_explainable(&entry));
    }
    let Some(current) = note_text(&vault, &entry)? else {
        return Ok(Outcome::SyncFirst {
            message: format!(
                "the file of '{}' is gone; a sync writes it again",
                entry.path
            ),
        });
    };
    if submission.base != body_hash(&current) {
        return Ok(Outcome::Changed {
            message: format!(
                "'{}' changed since you read its brief; read the brief again and write from it",
                entry.path
            ),
        });
    }
    let stale = changed_since_sync(root, &entry);
    if !stale.is_empty() {
        return Ok(Outcome::SyncFirst {
            message: format!(
                "the code under '{}' changed since the last sync ({}), so the facts of the note are old; sync, then read the brief again",
                entry.path,
                stale.join(", ")
            ),
        });
    }

    let mut issues: Vec<String> = Vec::new();
    let owned = kind.agent_sections();
    if submission.sections.is_empty() {
        issues.push(format!(
            "no section was sent; write at least one of: {}",
            owned.join(", ")
        ));
    }
    let mut agent: Vec<Section> = Vec::new();
    for written in &submission.sections {
        let heading = written.heading.trim();
        if !owned.contains(&heading) {
            issues.push(format!(
                "`{heading}` is not a section an agent writes in this kind of note; yours are: {}",
                owned.join(", ")
            ));
            continue;
        }
        if agent.iter().any(|s| s.heading == heading) {
            issues.push(format!("`{heading}` was sent twice"));
            continue;
        }
        review_section(kind, written, &mut issues);
        agent.push(Section {
            heading: heading.to_string(),
            body: written.text.trim().to_string(),
        });
    }

    let woven = cw_vault::weave(&current, &agent, kind);
    let registry = meta.registry()?;
    // The note with the agent's text must pass the check every note CodeWiki
    // writes passes. A fault the note had before is not the agent's: its
    // owner may have written it, and the agent cannot mend it.
    let faults = |text: &str| -> Vec<(&'static str, String)> {
        lint(text, &entry.path, kind, &registry, Mode::Strict)
            .into_iter()
            .filter(|issue| issue.severity == Severity::Error)
            .map(|issue| (issue.rule, issue.message))
            .collect()
    };
    // A fault is the same one wherever its line moved to.
    let unplaced = |message: &str| -> String {
        message
            .strip_prefix("line ")
            .and_then(|rest| rest.split_once(": "))
            .map_or(message, |(_, what)| what)
            .to_string()
    };
    let mut before: Vec<(&'static str, String)> = faults(&current)
        .into_iter()
        .map(|(rule, message)| (rule, unplaced(&message)))
        .collect();
    for (rule, message) in faults(&woven.text) {
        let key = (rule, unplaced(&message));
        match before.iter().position(|known| *known == key) {
            Some(index) => {
                before.swap_remove(index);
            }
            None => issues.push(format!("[{rule}] {}", key.1)),
        }
    }

    // The pictures the note has after this: those sent, and those of the
    // sections that were not sent.
    let (_, sections) = cw_vault::note::split(&woven.text);
    let mut pictures: Vec<Picture> = Vec::new();
    for picture in &submission.pictures {
        let section = picture.section.trim();
        if !owned.contains(&section) || !sections.iter().any(|s| s.heading == section) {
            issues.push(format!(
                "a picture names the section `{section}`; it must stand in a section of yours that the note has"
            ));
            continue;
        }
        if pictures.iter().any(|p| p.section == section) {
            issues.push(format!(
                "two pictures for `{section}`; a section has at most one"
            ));
            continue;
        }
        for issue in cw_illustrate::review(&picture.source) {
            issues.push(format!("picture for `{section}`: {issue}"));
        }
        pictures.push(Picture {
            section: section.to_string(),
            source: format!("{}\n", picture.source.trim()),
        });
    }
    for old in pictures_of(&vault, &entry) {
        let resent = agent.iter().any(|s| s.heading == old.section)
            || pictures.iter().any(|p| p.section == old.section);
        if !resent && sections.iter().any(|s| s.heading == old.section) {
            pictures.push(old);
        }
    }
    if pictures.len() > cw_illustrate::MAX_PICTURES {
        issues.push(format!(
            "the note would have {} pictures; at most {}",
            pictures.len(),
            cw_illustrate::MAX_PICTURES
        ));
    }
    if !issues.is_empty() {
        return Ok(Outcome::Issues { issues });
    }
    // In the order of the note's sections, as the app shows them.
    pictures.sort_by_key(|p| sections.iter().position(|s| s.heading == p.section));
    let pictures_file = cw_illustrate::serialize_pictures(&pictures);

    // What the explanation was written from: the note's sources and the
    // files the agent says it read, each as it reads now.
    let mut sources: Vec<SourceRef> = entry.sources.clone();
    for path in submission.read.iter().map(|p| tidy(p)).take(MAX_READ) {
        if sources.iter().any(|s| s.path == path) {
            continue;
        }
        if let Some(hash) = file_hash(root, &path) {
            sources.push(SourceRef {
                path,
                symbols: Vec::new(),
                hash,
            });
        }
    }
    // The sections that hold an agent's text: those of before that the
    // kind still has, and those of now.
    let before: &[String] = entry
        .explained
        .as_ref()
        .map(|e| e.sections.as_slice())
        .unwrap_or(&[]);
    let explained = Explained {
        sections: owned
            .iter()
            .filter(|h| before.iter().any(|b| b == *h) || woven.written.iter().any(|w| w == *h))
            .map(|h| h.to_string())
            .collect(),
        sources,
        commit: crate::git::head_commit(root),
    };
    let model = agent_name(submission.model.as_deref());

    if entry.origin == Origin::Adopted || body_hash(&current) != entry.body_hash {
        let filed = proposal::create_agent(
            &vault,
            &entry.id,
            &entry.path,
            &current,
            &woven.text,
            &format!("{model} explained this note"),
            Offer {
                explained,
                model: Some(model),
                pictures: pictures_file,
            },
        )?;
        return Ok(Outcome::Proposed {
            path: entry.path.clone(),
            proposal: filed.id,
            message: format!(
                "'{}' was edited by its owner, so it was not written; your text waits in the review queue for the owner to approve",
                entry.path
            ),
        });
    }

    let full = vault.join(&entry.path);
    std::fs::write(&full, &woven.text).map_err(|e| Error::io(&full, e))?;
    let saved = meta
        .notes
        .get_mut(&entry.id)
        .ok_or_else(|| Error::not_found(format!("note '{}'", entry.path)))?;
    record(&vault, saved, &woven.text, explained, model, &pictures_file)?;
    let sections = saved
        .explained
        .as_ref()
        .map(|e| e.sections.clone())
        .unwrap_or_default();
    meta.save(&vault)?;
    Ok(Outcome::Saved {
        path: entry.path,
        sections,
        pictures: pictures.len(),
        waiting: count(&all_waiting(&vault, &meta)),
    })
}

/// Record on `entry` that an agent explained the note, and keep its
/// pictures beside it. `text` is the note as it now is on disk.
fn record(
    vault: &Path,
    entry: &mut NoteEntry,
    text: &str,
    explained: Explained,
    model: String,
    pictures_file: &str,
) -> Result<()> {
    let relative = illustration_path(&entry.path);
    let full = vault.join(&relative);
    if pictures_file.trim().is_empty() {
        match std::fs::remove_file(&full) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io(&full, e)),
        }
        entry.illustration = None;
    } else {
        std::fs::write(&full, pictures_file).map_err(|e| Error::io(&full, e))?;
        entry.illustration = Some(relative);
    }
    entry.body_hash = body_hash(text);
    entry.explained = Some(explained);
    entry.model = Some(model);
    entry.behind.clear();
    Ok(())
}

/// Approve the pending proposal `id`: write the note, and for an agent's
/// proposal its pictures and the record of who explained it.
///
/// After a sync's proposal the note counts as reviewed for that input, so
/// the same change is not proposed again. After an agent's proposal the
/// note stays its owner's: it was edited by hand and nothing overwrites
/// it, so the next change of its facts is a proposal again.
pub fn approve(root: &Path, id: &str) -> Result<proposal::Proposal> {
    let vault = root.join(VAULT_DIR);
    // Read before the approval, which may be refused.
    let pictures = proposal::pictures_text(&vault, id)?;
    let done = proposal::approve(&vault, id)?;
    let mut meta = VaultMeta::load(&vault)?;
    let Some(entry) = meta.notes.get_mut(&done.note_id) else {
        return Ok(done);
    };
    match (done.kind, &done.explained) {
        (ProposalKind::Agent, Some(explained)) => {
            // The owner's hand stays on the note: the hash on record is
            // not the new text's.
            let owner_hash = entry.body_hash.clone();
            let text = proposal::proposed_text(&vault, id)?;
            let model = agent_name(done.model.as_deref());
            record(&vault, entry, &text, explained.clone(), model, &pictures)?;
            entry.body_hash = owner_hash;
        }
        _ => entry.input_hash = done.input_hash.clone(),
    }
    meta.save(&vault)?;
    Ok(done)
}

/// Reject the pending proposal `id`. A sync's input is recorded, so the
/// same change is not proposed again. An agent's text is dropped, and an
/// agent may offer another.
pub fn reject(root: &Path, id: &str) -> Result<proposal::Proposal> {
    let vault = root.join(VAULT_DIR);
    let done = proposal::reject(&vault, id)?;
    if done.kind == ProposalKind::Sync {
        let mut meta = VaultMeta::load(&vault)?;
        if let Some(entry) = meta.notes.get_mut(&done.note_id) {
            entry.rejected_inputs.push(done.input_hash.clone());
            meta.save(&vault)?;
        }
    }
    Ok(done)
}
