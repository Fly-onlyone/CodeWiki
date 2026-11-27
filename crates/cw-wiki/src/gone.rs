//! Notes whose subject is gone from the repository: which of them leave the
//! vault in a sync, which stay, and why.
//!
//! A note nobody wrote in is the rules' alone and goes with its subject at
//! once. A note that holds written work, an agent's explanation or its
//! owner's edits, goes only when nothing is lost by it: the deletion of its
//! subject is committed, and git holds the note as it is now.

use std::cell::OnceCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use cw_core::{Error, Result, VAULT_DIR};
use cw_vault::proposal::{self, Proposal, ProposalStatus};
use cw_vault::{Gone, NoteEntry, VaultMeta, body_hash};

use crate::generate::illustration_path;
use crate::git;

/// What git knows about a repository, each answer asked for once and only
/// when a note needs it: most syncs remove nothing.
pub(crate) struct Git<'a> {
    root: &'a Path,
    recoverable: OnceCell<HashSet<String>>,
    changed: OnceCell<Option<HashSet<String>>>,
    head: OnceCell<Option<HashSet<String>>>,
    history: OnceCell<HashSet<String>>,
    history_folders: OnceCell<HashSet<String>>,
    moves: OnceCell<HashMap<String, String>>,
}

impl<'a> Git<'a> {
    pub(crate) fn new(root: &'a Path) -> Self {
        Self {
            root,
            recoverable: OnceCell::new(),
            changed: OnceCell::new(),
            head: OnceCell::new(),
            history: OnceCell::new(),
            history_folders: OnceCell::new(),
            moves: OnceCell::new(),
        }
    }

    /// The repository folder this view is of.
    pub(crate) fn root(&self) -> &Path {
        self.root
    }

    /// True when a commit once held `path`, as a file or as a folder.
    pub(crate) fn knew(&self, path: &str) -> bool {
        self.history().contains(path)
            || self
                .history_folders
                .get_or_init(|| {
                    let mut folders = HashSet::new();
                    for file in self.history() {
                        let mut rest = file.as_str();
                        while let Some((folder, _)) = rest.rsplit_once('/') {
                            if !folders.insert(folder.to_string()) {
                                break;
                            }
                            rest = folder;
                        }
                    }
                    folders
                })
                .contains(path)
    }

    /// True when the commit at `HEAD` still has `path`, as a file or as a
    /// folder: its deletion is not committed.
    pub(crate) fn has_at_head(&self, path: &str) -> bool {
        let inside = format!("{path}/");
        self.head()
            .is_some_and(|head| head.contains(path) || head.iter().any(|f| f.starts_with(&inside)))
    }

    /// See [`git::recoverable`].
    pub(crate) fn recoverable(&self) -> &HashSet<String> {
        self.recoverable.get_or_init(|| git::recoverable(self.root))
    }

    /// See [`git::changed`].
    pub(crate) fn changed(&self) -> Option<&HashSet<String>> {
        self.changed
            .get_or_init(|| git::changed(self.root))
            .as_ref()
    }

    /// See [`git::at_head`].
    pub(crate) fn head(&self) -> Option<&HashSet<String>> {
        self.head.get_or_init(|| git::at_head(self.root)).as_ref()
    }

    /// See [`git::history_paths`].
    pub(crate) fn history(&self) -> &HashSet<String> {
        self.history.get_or_init(|| git::history_paths(self.root))
    }

    /// See [`git::moves`].
    pub(crate) fn moves(&self) -> &HashMap<String, String> {
        self.moves.get_or_init(|| git::moves(self.root))
    }

    /// True when git holds the note of `entry`, and its pictures, as they
    /// are on disk now.
    pub(crate) fn holds(&self, entry: &NoteEntry) -> bool {
        let safe = self.recoverable();
        safe.contains(entry.path.as_str())
            && (entry.illustration.is_none()
                || safe.contains(illustration_path(&entry.path).as_str()))
    }
}

/// What a sync does with a note whose subject is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Delete it now.
    Remove,
    /// Leave it, for this reason.
    Keep(Gone),
}

/// The verdict for a note CodeWiki wrote that holds written work and has no
/// place in the plan any more.
pub(crate) fn of_generated(entry: &NoteEntry, git: &Git) -> Verdict {
    // The code it was written from is still being changed: a file deleted
    // and not committed may come back, and its note with it.
    let unsettled = git.changed().is_some_and(|changed| {
        entry
            .sources
            .iter()
            .any(|source| changed.contains(source.path.as_str()))
    });
    if unsettled {
        Verdict::Keep(Gone::Uncommitted)
    } else if git.holds(entry) {
        Verdict::Remove
    } else {
        Verdict::Keep(Gone::Unsaved)
    }
}

/// The repository path a code span of a note names, if it has the shape of
/// one: `src/a.rs`, `src/a.rs:12`, `./src/`, `src\a.rs`. Whether such a
/// path ever existed is the caller's question.
fn path_in(span: &str) -> Option<String> {
    // What stands between two fences of a code block is no code span.
    if span.contains('\n') {
        return None;
    }
    let span = span.trim();
    if span.is_empty() || span.len() > 260 {
        return None;
    }
    // A line number or a range of lines after the path.
    let span = match span.rsplit_once(':') {
        Some((path, lines))
            if !lines.is_empty() && lines.chars().all(|c| c.is_ascii_digit() || c == '-') =>
        {
            path
        }
        _ => span,
    };
    let path = span.replace('\\', "/");
    let path = path.trim_start_matches("./").trim_end_matches('/');
    let inside = !path.is_empty()
        && !path.contains(':')
        && !path.starts_with('/')
        && path.split('/').all(|part| part != ".." && !part.is_empty());
    // A bare word names no path; a path has a folder or an extension.
    (inside && (path.contains('/') || path.contains('.'))).then(|| path.to_string())
}

/// The code spans of a note, each with whether it stands inside backticks.
fn spans(text: &str) -> impl Iterator<Item = (bool, &str)> {
    text.split('`')
        .enumerate()
        .map(|(place, part)| (place % 2 == 1, part))
}

/// The repository paths a note names in its text, by what became of them.
/// A span that never named a path of the repository is in none of them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Cited {
    /// Paths that exist.
    pub alive: Vec<String>,
    /// Paths git saw move, each with where it is now.
    pub moved: Vec<(String, String)>,
    /// Paths a commit once held and that are gone.
    pub dead: Vec<String>,
}

impl Cited {
    /// Every path, in order, for the note's record.
    pub(crate) fn all(&self) -> Vec<String> {
        let mut all: Vec<String> = self
            .alive
            .iter()
            .chain(self.moved.iter().map(|(old, _)| old))
            .chain(&self.dead)
            .cloned()
            .collect();
        all.sort();
        all.dedup();
        all
    }
}

/// Read the paths the note `text` names anywhere in it.
pub(crate) fn cited(text: &str, git: &Git) -> Cited {
    let mut found = Cited::default();
    let mut seen: HashSet<String> = HashSet::new();
    for path in spans(text)
        .filter(|(code, _)| *code)
        .filter_map(|(_, span)| path_in(span))
    {
        if !seen.insert(path.clone()) {
            continue;
        }
        if git.root().join(&path).exists() {
            found.alive.push(path);
        } else if let Some(end) = git.moves().get(&path)
            && git.root().join(end).exists()
        {
            found.moved.push((path, end.clone()));
        } else if git.knew(&path) {
            found.dead.push(path);
        }
    }
    found.alive.sort();
    found.moved.sort();
    found.dead.sort();
    found
}

/// The note `text` with each moved path it names replaced by where the file
/// is now. Only code spans change, and only the path in them.
pub(crate) fn repath(text: &str, moved: &[(String, String)]) -> String {
    let mut out = String::with_capacity(text.len());
    for (place, (code, part)) in spans(text).enumerate() {
        if place > 0 {
            out.push('`');
        }
        let end = path_in(part)
            .filter(|_| code)
            .and_then(|path| moved.iter().find(|(old, _)| *old == path));
        match end {
            // As the note wrote the path, or with the other slash.
            Some((old, new)) if part.contains(old.as_str()) => {
                out.push_str(&part.replacen(old.as_str(), new, 1));
            }
            Some((old, new)) => {
                out.push_str(&part.replace('\\', "/").replacen(old.as_str(), new, 1));
            }
            None => out.push_str(part),
        }
    }
    out
}

/// What a sync finds about a note its owner wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hand {
    /// It names no path of the repository: nothing says what it is about,
    /// so nothing says that is gone.
    Unjudged,
    /// Every path it names exists.
    Alive,
    /// Some of the paths it names are gone and some exist.
    PartlyGone,
    /// None of the paths it names exists.
    Gone(Gone),
}

/// The verdict for a note its owner wrote, from the paths it names.
pub(crate) fn of_adopted(entry: &NoteEntry, cited: &Cited, git: &Git) -> Hand {
    let lives = !cited.alive.is_empty() || !cited.moved.is_empty();
    match (lives, cited.dead.is_empty()) {
        (false, true) => Hand::Unjudged,
        (true, true) => Hand::Alive,
        (true, false) => Hand::PartlyGone,
        (false, false) => Hand::Gone(if entry.gone == Some(Gone::History) {
            // Its owner decided to keep it; that stands.
            Gone::History
        } else if cited.dead.iter().any(|path| git.has_at_head(path)) {
            Gone::Uncommitted
        } else if git.holds(entry) {
            Gone::Pending
        } else {
            Gone::Unsaved
        }),
    }
}

/// How a line of [`NoteEntry::behind`] starts that says a link of the
/// agent's became plain words because its note was removed.
pub(crate) const UNLINKED: &str = "a note it linked was removed: ";

/// A note that left the vault, with its text as it was.
pub(crate) struct Left {
    /// Vault-relative path.
    pub path: String,
    pub text: String,
}

/// What [`after_removal`] changed beside the notes that left.
#[derive(Debug, Default)]
pub(crate) struct Cleanup {
    /// Notes in which a link to a removed note became plain words.
    pub unlinked: Vec<String>,
    /// Attachments deleted because only removed notes showed them.
    pub attachments: Vec<String>,
}

/// Folder of a vault for the files notes show. CodeWiki writes no note
/// there and judges none.
const ATTACHMENTS: &str = "_Attachments";

/// Tidy the vault after the notes `left` were deleted from it and from
/// `meta`, which the caller saves.
///
/// A link to one of them becomes plain words in every note that stays. In
/// a note an agent explained and nobody edited, that is the agent's text
/// changing, so the note is marked behind. A note its owner wrote or edited
/// is changed only when git holds it as it is. Only links to the notes that
/// just left are touched: any other link that leads nowhere is its owner's
/// to mend. An attachment that only removed notes showed is deleted, when
/// git holds it.
pub(crate) fn after_removal(
    vault: &Path,
    meta: &mut VaultMeta,
    git: &Git,
    left: &[Left],
) -> Result<Cleanup> {
    let mut cleanup = Cleanup::default();
    if left.is_empty() {
        return Ok(cleanup);
    }
    // A link names a note by its name or by its path, without `.md`.
    let mut removed: HashSet<String> = HashSet::new();
    let mut shown: Vec<String> = Vec::new();
    for note in left {
        let path = note.path.trim_end_matches(".md").to_lowercase();
        removed.insert(path.rsplit('/').next().unwrap_or(&path).to_string());
        removed.insert(path);
        for link in cw_vault::Note::parse(&note.text).links {
            let file = if link.target.starts_with(&format!("{ATTACHMENTS}/")) {
                link.target.clone()
            } else {
                format!("{ATTACHMENTS}/{}", link.target)
            };
            if vault.join(&file).is_file() && !shown.contains(&file) {
                shown.push(file);
            }
        }
    }
    let registry = meta.registry()?;
    let mut still_shown: HashSet<String> = HashSet::new();
    for entry in meta.notes.values_mut() {
        let full = vault.join(&entry.path);
        let Ok(text) = std::fs::read_to_string(&full) else {
            continue;
        };
        for file in &shown {
            let name = file.rsplit('/').next().unwrap_or(file);
            if text.contains(name) {
                still_shown.insert(file.clone());
            }
        }
        // A note kept after its subject went is left as it is.
        if entry.gone.is_some() {
            continue;
        }
        let dead = std::cell::RefCell::new(Vec::<String>::new());
        let alive = |target: &str| {
            let name = target.trim().trim_end_matches(".md");
            let gone = registry.resolve(name, &entry.path).is_none()
                && removed.contains(&name.to_lowercase());
            if gone && !dead.borrow().iter().any(|d| d == name) {
                dead.borrow_mut().push(name.to_string());
            }
            !gone
        };
        let by_hand =
            entry.origin == cw_vault::Origin::Adopted || body_hash(&text) != entry.body_hash;
        let changed = if by_hand {
            cw_vault::unlink_dead(&text, alive)
        } else {
            // Rules write their own sections again at the next sync; only
            // the agent's words are changed here.
            let Some(explained) = &entry.explained else {
                continue;
            };
            let mut agent = cw_vault::extract(&text, entry.kind, &explained.sections);
            for section in &mut agent {
                section.body = cw_vault::unlink_dead(&section.body, alive);
            }
            cw_vault::weave(&text, &agent, entry.kind).text
        };
        let dead = dead.into_inner();
        if dead.is_empty() || changed == text || (by_hand && !git.holds(entry)) {
            continue;
        }
        std::fs::write(&full, &changed).map_err(|e| Error::io(&full, e))?;
        if by_hand {
            // A note CodeWiki wrote and its owner edited stays its owner's:
            // the hash on record is still not the text's.
            if entry.origin == cw_vault::Origin::Adopted {
                entry.body_hash = body_hash(&changed);
            }
        } else {
            entry.body_hash = body_hash(&changed);
            for name in dead {
                let line = format!("{UNLINKED}`{name}`");
                if !entry.behind.contains(&line) {
                    entry.behind.push(line);
                }
            }
        }
        cleanup.unlinked.push(entry.path.clone());
    }
    for file in shown {
        if !still_shown.contains(&file) && git.recoverable().contains(&file) {
            let full = vault.join(&file);
            std::fs::remove_file(&full).map_err(|e| Error::io(&full, e))?;
            cleanup.attachments.push(file);
        }
    }
    cleanup.unlinked.sort();
    cleanup.attachments.sort();
    Ok(cleanup)
}

/// Remove every folder of the vault that holds nothing, deepest first: a
/// folder whose notes all left is no part of the wiki. Folders that are not
/// for notes are left alone, empty or not.
pub(crate) fn remove_empty_folders(vault: &Path) -> Result<()> {
    fn sweep(dir: &Path) -> Result<bool> {
        let mut empty = true;
        for entry in std::fs::read_dir(dir).map_err(|e| Error::io(dir, e))? {
            let entry = entry.map_err(|e| Error::io(dir, e))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let for_notes = !name.starts_with('.') && name != "_Templates" && name != ATTACHMENTS;
            // A folder that cannot be removed got something meanwhile, and
            // stays.
            if path.is_dir() && for_notes && sweep(&path)? && std::fs::remove_dir(&path).is_ok() {
                continue;
            }
            empty = false;
        }
        Ok(empty)
    }
    if vault.is_dir() {
        sweep(vault)?;
    }
    Ok(())
}

/// A vault path as the manifest writes it.
fn tidy(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_matches('/')
        .to_string()
}

fn pending_retire(vault: &Path, id: &str) -> Result<Proposal> {
    let found = proposal::get(vault, id)?;
    if found.status != ProposalStatus::Pending {
        return Err(Error::invalid(format!("proposal {id} is not pending")));
    }
    Ok(found)
}

/// Approve the retire proposal `id` of the repository at `root`: delete the
/// notes it names, except those in `keep` (vault paths), which stay as
/// history.
///
/// A note is deleted only if it is still the note the proposal saw, still
/// marked as waiting for this decision, and held by git as it is. One that
/// fails any of these stays and is judged again by the next sync.
pub(crate) fn retire(root: &Path, id: &str, keep: &[String]) -> Result<Proposal> {
    let vault = root.join(VAULT_DIR);
    let pending = pending_retire(&vault, id)?;
    let keep: HashSet<String> = keep.iter().map(|path| tidy(path)).collect();
    let mut meta = VaultMeta::load(&vault)?;
    let git = Git::new(root);
    let mut left: Vec<Left> = Vec::new();
    for note in &pending.retire {
        let Some(entry) = meta.notes.get_mut(&note.note_id) else {
            continue;
        };
        if entry.gone != Some(Gone::Pending) {
            continue;
        }
        if keep.contains(&note.path) {
            entry.gone = Some(Gone::History);
            continue;
        }
        let full = vault.join(&entry.path);
        let Ok(text) = std::fs::read_to_string(&full) else {
            continue;
        };
        if body_hash(&text) != note.base_hash || !git.holds(entry) {
            continue;
        }
        std::fs::remove_file(&full).map_err(|e| Error::io(&full, e))?;
        let _ = std::fs::remove_file(vault.join(illustration_path(&entry.path)));
        left.push(Left {
            path: entry.path.clone(),
            text,
        });
        meta.notes.remove(&note.note_id);
    }
    after_removal(&vault, &mut meta, &git, &left)?;
    remove_empty_folders(&vault)?;
    meta.save(&vault)?;
    proposal::close_retire(&vault, id, ProposalStatus::Approved)
}

/// Reject the retire proposal `id`: every note it names stays, as history,
/// and is not proposed again.
pub(crate) fn keep_all(root: &Path, id: &str) -> Result<Proposal> {
    let vault = root.join(VAULT_DIR);
    let pending = pending_retire(&vault, id)?;
    let mut meta = VaultMeta::load(&vault)?;
    for note in &pending.retire {
        if let Some(entry) = meta.notes.get_mut(&note.note_id)
            && entry.gone == Some(Gone::Pending)
        {
            entry.gone = Some(Gone::History);
        }
    }
    meta.save(&vault)?;
    proposal::close_retire(&vault, id, ProposalStatus::Rejected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_span_names_a_path_only_when_it_has_the_shape_of_one() {
        for (span, path) in [
            ("src/a.rs", Some("src/a.rs")),
            (" src/a.rs:12 ", Some("src/a.rs")),
            ("src/a.rs:12-40", Some("src/a.rs")),
            ("./backend/app/", Some("backend/app")),
            ("src\\win\\a.rs", Some("src/win/a.rs")),
            ("pyproject.toml", Some("pyproject.toml")),
            ("src/old name.rs", Some("src/old name.rs")),
            ("Price", None),
            ("", None),
            ("../outside/a.rs", None),
            ("/etc/passwd", None),
            ("C:/Users/a.rs", None),
            ("a//b.rs", None),
            ("two\nlines.rs", None),
        ] {
            assert_eq!(path_in(span).as_deref(), path, "{span:?}");
        }
    }

    #[test]
    fn only_the_moved_path_changes_and_only_in_a_code_span() {
        let moved = vec![(
            "legacy/moved.cfg".to_string(),
            "tools/moved.cfg".to_string(),
        )];
        let text = "See `legacy/moved.cfg`, `legacy\\moved.cfg:3` and `legacy/moved.cfg.bak`.\n\nNot code: legacy/moved.cfg\n\n```\nlegacy/moved.cfg\n```\n";
        assert_eq!(
            repath(text, &moved),
            "See `tools/moved.cfg`, `tools/moved.cfg:3` and `legacy/moved.cfg.bak`.\n\nNot code: legacy/moved.cfg\n\n```\nlegacy/moved.cfg\n```\n"
        );
        assert_eq!(repath("no code here", &moved), "no code here");
    }
}
