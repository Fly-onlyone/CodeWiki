//! The review queue: proposed changes to human-edited notes.
//!
//! A proposal stores the full proposed text plus a unified diff for reading.
//! Nothing in the vault changes until a human approves it, and approval only
//! applies when the note is still the version the proposal was written against.
//!
//! Two things propose. A sync does when the code under a note changed. A
//! coding agent does when it explained a note its owner had edited: its
//! text and pictures wait here instead of replacing the owner's words.

use std::path::{Path, PathBuf};

use cw_core::{Error, Result, sha256_hex};
use serde::{Deserialize, Serialize};
use similar::TextDiff;

use crate::manifest::{Explained, VaultMeta, body_hash};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected,
    /// The note changed after the proposal was written.
    Superseded,
}

/// What made a proposal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalKind {
    /// A sync: the code under the note changed.
    #[default]
    Sync,
    /// A coding agent that explained the note.
    Agent,
}

/// What an agent's proposal carries beside the note's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    /// What the agent explained the note from; recorded on the note when
    /// the proposal is approved.
    pub explained: Explained,
    /// The agent, as it names itself.
    pub model: Option<String>,
    /// The pictures file the note would get; empty for no pictures.
    pub pictures: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub note_id: String,
    /// Vault-relative path of the note.
    pub note_path: String,
    /// Body hash of the note the proposal was written against; empty when the
    /// proposal creates a new note.
    pub base_hash: String,
    /// Input hash of the generation that produced the proposal.
    pub input_hash: String,
    pub reason: String,
    pub status: ProposalStatus,
    /// What made it. A proposal written before agents explained notes is a
    /// sync's.
    #[serde(default)]
    pub kind: ProposalKind,
    /// For an agent's proposal: what the note was explained from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explained: Option<Explained>,
    /// For an agent's proposal: the agent, as it names itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Whether the proposal carries pictures ([`pictures_text`]).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pictures: bool,
}

fn proposals_dir(vault: &Path) -> PathBuf {
    VaultMeta::dir(vault).join("proposals")
}

/// Unified diff between two note texts, for display.
pub fn unified_diff(old: &str, new: &str, path: &str) -> String {
    TextDiff::from_lines(old, new)
        .unified_diff()
        .context_radius(3)
        .header(&format!("a/{path}"), &format!("b/{path}"))
        .to_string()
}

fn read_json(path: &Path) -> Result<Proposal> {
    let text = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
    serde_json::from_str(&text).map_err(|e| Error::external("parsing a proposal", e))
}

fn write_json(vault: &Path, proposal: &Proposal) -> Result<()> {
    let path = proposals_dir(vault).join(format!("{}.json", proposal.id));
    let text = serde_json::to_string_pretty(proposal)
        .map_err(|e| Error::external("encoding a proposal", e))?;
    std::fs::write(&path, text + "\n").map_err(|e| Error::io(path, e))
}

/// Queue a sync's proposal. `current` is the note text on disk, empty for a
/// new note. An earlier pending proposal of a sync for the same note is
/// replaced; one of an agent stays.
pub fn create(
    vault: &Path,
    note_id: &str,
    note_path: &str,
    current: &str,
    proposed: &str,
    input_hash: &str,
    reason: &str,
) -> Result<Proposal> {
    create_as(
        vault,
        (note_id, note_path),
        current,
        proposed,
        input_hash,
        reason,
        None,
    )
}

/// Queue an agent's proposal: the note with the agent's sections woven in,
/// and what comes with it. An earlier pending proposal of an agent for the
/// same note is replaced; one of a sync stays, since it says something
/// else: that the facts changed.
pub fn create_agent(
    vault: &Path,
    note_id: &str,
    note_path: &str,
    current: &str,
    proposed: &str,
    reason: &str,
    offer: Offer,
) -> Result<Proposal> {
    // What is offered names the proposal: the same offer twice is one.
    let hash = sha256_hex(format!("agent\n{proposed}\n{}", offer.pictures));
    create_as(
        vault,
        (note_id, note_path),
        current,
        proposed,
        &hash,
        reason,
        Some(offer),
    )
}

fn create_as(
    vault: &Path,
    (note_id, note_path): (&str, &str),
    current: &str,
    proposed: &str,
    input_hash: &str,
    reason: &str,
    offer: Option<Offer>,
) -> Result<Proposal> {
    let kind = if offer.is_some() {
        ProposalKind::Agent
    } else {
        ProposalKind::Sync
    };
    let dir = proposals_dir(vault);
    std::fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    for old in list(vault)? {
        if old.note_id == note_id && old.status == ProposalStatus::Pending && old.kind == kind {
            remove(vault, &old.id)?;
        }
    }
    let short = &input_hash[..input_hash.len().min(12)];
    let pictures = offer
        .as_ref()
        .map(|o| o.pictures.clone())
        .unwrap_or_default();
    let proposal = Proposal {
        // The two kinds never share an id, so neither overwrites the other.
        id: match kind {
            ProposalKind::Sync => format!("{}-{short}", slug(note_id)),
            ProposalKind::Agent => format!("{}-agent-{short}", slug(note_id)),
        },
        kind,
        explained: offer.as_ref().map(|o| o.explained.clone()),
        model: offer.and_then(|o| o.model),
        pictures: !pictures.trim().is_empty(),
        note_id: note_id.to_string(),
        note_path: note_path.to_string(),
        base_hash: if current.is_empty() {
            String::new()
        } else {
            body_hash(current)
        },
        input_hash: input_hash.to_string(),
        reason: reason.to_string(),
        status: ProposalStatus::Pending,
    };
    write_json(vault, &proposal)?;
    let write = |ext: &str, text: &str| {
        let path = dir.join(format!("{}.{ext}", proposal.id));
        std::fs::write(&path, text).map_err(|e| Error::io(path, e))
    };
    write("md", proposed)?;
    write("patch", &unified_diff(current, proposed, note_path))?;
    if proposal.pictures {
        write(PICTURES, &pictures)?;
    }
    Ok(proposal)
}

/// All proposals, sorted by id.
pub fn list(vault: &Path) -> Result<Vec<Proposal>> {
    let dir = proposals_dir(vault);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(dir, e)),
    };
    let mut out = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| Error::io(&dir, e))?.path();
        if path.extension().is_some_and(|e| e == "json") {
            out.push(read_json(&path)?);
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

pub fn get(vault: &Path, id: &str) -> Result<Proposal> {
    check_id(id)?;
    let path = proposals_dir(vault).join(format!("{id}.json"));
    if !path.exists() {
        return Err(Error::not_found(format!("proposal {id}")));
    }
    read_json(&path)
}

/// Proposed full text of the note.
pub fn proposed_text(vault: &Path, id: &str) -> Result<String> {
    check_id(id)?;
    let path = proposals_dir(vault).join(format!("{id}.md"));
    std::fs::read_to_string(&path).map_err(|e| Error::io(path, e))
}

pub fn patch_text(vault: &Path, id: &str) -> Result<String> {
    check_id(id)?;
    let path = proposals_dir(vault).join(format!("{id}.patch"));
    std::fs::read_to_string(&path).map_err(|e| Error::io(path, e))
}

/// The pictures file an agent's proposal would give the note; empty when
/// it carries none.
pub fn pictures_text(vault: &Path, id: &str) -> Result<String> {
    check_id(id)?;
    let path = proposals_dir(vault).join(format!("{id}.{PICTURES}"));
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(Error::io(path, e)),
    }
}

/// Extension of the file that holds a proposal's pictures.
const PICTURES: &str = "openui";

/// File-name-safe form of a note id such as `type:cw_core::Error`.
fn slug(note_id: &str) -> String {
    note_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Ids come from API callers; never let one name a path outside the folder.
fn check_id(id: &str) -> Result<()> {
    let ok = !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'));
    if ok {
        Ok(())
    } else {
        Err(Error::invalid(format!("invalid proposal id '{id}'")))
    }
}

fn remove(vault: &Path, id: &str) -> Result<()> {
    for ext in ["json", "md", "patch", PICTURES] {
        let path = proposals_dir(vault).join(format!("{id}.{ext}"));
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io(path, e)),
        }
    }
    Ok(())
}

/// Apply a pending proposal: write the note and record the outcome. Fails
/// with the proposal marked `Superseded` when the note changed meanwhile.
pub fn approve(vault: &Path, id: &str) -> Result<Proposal> {
    let mut proposal = get(vault, id)?;
    if proposal.status != ProposalStatus::Pending {
        return Err(Error::invalid(format!("proposal {id} is not pending")));
    }
    let note_path = vault.join(&proposal.note_path);
    let current = match std::fs::read_to_string(&note_path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(Error::io(note_path, e)),
    };
    let current_hash = if current.is_empty() {
        String::new()
    } else {
        body_hash(&current)
    };
    if current_hash != proposal.base_hash {
        proposal.status = ProposalStatus::Superseded;
        write_json(vault, &proposal)?;
        return Err(Error::invalid(format!(
            "note {} changed since proposal {id} was written; a new proposal is needed",
            proposal.note_path
        )));
    }
    let text = proposed_text(vault, id)?;
    if let Some(parent) = note_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    std::fs::write(&note_path, &text).map_err(|e| Error::io(&note_path, e))?;
    proposal.status = ProposalStatus::Approved;
    write_json(vault, &proposal)?;
    Ok(proposal)
}

/// Reject a pending proposal. The caller records `input_hash` on the note
/// entry so the same change is not proposed again.
pub fn reject(vault: &Path, id: &str) -> Result<Proposal> {
    let mut proposal = get(vault, id)?;
    if proposal.status != ProposalStatus::Pending {
        return Err(Error::invalid(format!("proposal {id} is not pending")));
    }
    proposal.status = ProposalStatus::Rejected;
    write_json(vault, &proposal)?;
    Ok(proposal)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD: &str = "# Note\n\n> Summary.\n\nMy own words.\n";
    const NEW: &str = "# Note\n\n> Summary.\n\nMy own words.\n\n## Gotchas\n\n- New fact.\n";

    fn vault_with_note() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("01-Core")).unwrap();
        std::fs::write(dir.path().join("01-Core/Note.md"), OLD).unwrap();
        dir
    }

    fn queue(vault: &Path, input: &str) -> Proposal {
        create(
            vault,
            "note",
            "01-Core/Note.md",
            OLD,
            NEW,
            input,
            "src/note.rs changed",
        )
        .unwrap()
    }

    #[test]
    fn creating_does_not_touch_the_note() {
        let dir = vault_with_note();
        let p = queue(dir.path(), "inputhash0001");
        assert_eq!(p.status, ProposalStatus::Pending);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("01-Core/Note.md")).unwrap(),
            OLD
        );
        assert_eq!(proposed_text(dir.path(), &p.id).unwrap(), NEW);
        let patch = patch_text(dir.path(), &p.id).unwrap();
        assert!(patch.contains("+## Gotchas") && patch.contains("a/01-Core/Note.md"));
    }

    #[test]
    fn approving_writes_the_note() {
        let dir = vault_with_note();
        let p = queue(dir.path(), "inputhash0001");
        let done = approve(dir.path(), &p.id).unwrap();
        assert_eq!(done.status, ProposalStatus::Approved);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("01-Core/Note.md")).unwrap(),
            NEW
        );
        assert!(approve(dir.path(), &p.id).is_err(), "cannot approve twice");
    }

    #[test]
    fn approval_refuses_when_the_note_changed() {
        let dir = vault_with_note();
        let p = queue(dir.path(), "inputhash0001");
        let edited = "# Note\n\nEdited again by hand.\n";
        std::fs::write(dir.path().join("01-Core/Note.md"), edited).unwrap();
        assert!(approve(dir.path(), &p.id).is_err());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("01-Core/Note.md")).unwrap(),
            edited
        );
        assert_eq!(
            get(dir.path(), &p.id).unwrap().status,
            ProposalStatus::Superseded
        );
    }

    #[test]
    fn rejecting_leaves_the_note_alone() {
        let dir = vault_with_note();
        let p = queue(dir.path(), "inputhash0001");
        assert_eq!(
            reject(dir.path(), &p.id).unwrap().status,
            ProposalStatus::Rejected
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("01-Core/Note.md")).unwrap(),
            OLD
        );
    }

    #[test]
    fn a_newer_proposal_replaces_the_pending_one() {
        let dir = vault_with_note();
        queue(dir.path(), "inputhash0001");
        let second = queue(dir.path(), "inputhash0002");
        let all = list(dir.path()).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, second.id);
    }

    fn offer(pictures: &str) -> Offer {
        Offer {
            explained: Explained {
                sections: vec!["Gotchas".into()],
                sources: vec![],
                commit: Some("abc".into()),
            },
            model: Some("claude-code".into()),
            pictures: pictures.into(),
        }
    }

    #[test]
    fn an_agents_proposal_carries_what_it_explained_and_its_pictures() {
        let dir = vault_with_note();
        let picture = "# section: Gotchas\nroot = Picture([c])\nc = Callout(\"New fact.\")\n";
        let p = create_agent(
            dir.path(),
            "note",
            "01-Core/Note.md",
            OLD,
            NEW,
            "an agent explained this note",
            offer(picture),
        )
        .unwrap();
        assert_eq!(p.kind, ProposalKind::Agent);
        assert!(p.id.starts_with("note-agent-"), "{}", p.id);
        assert!(p.pictures);
        assert_eq!(p.model.as_deref(), Some("claude-code"));
        assert_eq!(p.explained.as_ref().unwrap().sections, ["Gotchas"]);
        // Read back from the queue, it is the same.
        assert_eq!(get(dir.path(), &p.id).unwrap(), p);
        assert_eq!(pictures_text(dir.path(), &p.id).unwrap(), picture);
        assert_eq!(proposed_text(dir.path(), &p.id).unwrap(), NEW);
        // The note itself is untouched until someone approves.
        assert_eq!(
            std::fs::read_to_string(dir.path().join("01-Core/Note.md")).unwrap(),
            OLD
        );
        // The same offer twice is one proposal.
        let again = create_agent(
            dir.path(),
            "note",
            "01-Core/Note.md",
            OLD,
            NEW,
            "an agent explained this note",
            offer(picture),
        )
        .unwrap();
        assert_eq!(again.id, p.id);
        assert_eq!(list(dir.path()).unwrap().len(), 1);

        // One without pictures keeps no pictures file.
        let plain = create_agent(
            dir.path(),
            "note",
            "01-Core/Note.md",
            OLD,
            NEW,
            "an agent explained this note",
            offer(""),
        )
        .unwrap();
        assert!(!plain.pictures);
        assert_eq!(pictures_text(dir.path(), &plain.id).unwrap(), "");
        // It replaced the agent's earlier one, pictures file and all.
        assert_eq!(list(dir.path()).unwrap().len(), 1);
        assert_eq!(pictures_text(dir.path(), &p.id).unwrap(), "");
    }

    #[test]
    fn a_sync_and_an_agent_do_not_replace_each_other() {
        let dir = vault_with_note();
        let sync = queue(dir.path(), "inputhash0001");
        assert_eq!(sync.kind, ProposalKind::Sync);
        let agent = create_agent(
            dir.path(),
            "note",
            "01-Core/Note.md",
            OLD,
            NEW,
            "an agent explained this note",
            offer(""),
        )
        .unwrap();
        let ids: Vec<String> = list(dir.path())
            .unwrap()
            .into_iter()
            .map(|p| p.id)
            .collect();
        assert!(ids.contains(&sync.id) && ids.contains(&agent.id), "{ids:?}");
        // A newer sync replaces the sync's, and leaves the agent's.
        let newer = queue(dir.path(), "inputhash0002");
        let ids: Vec<String> = list(dir.path())
            .unwrap()
            .into_iter()
            .map(|p| p.id)
            .collect();
        assert_eq!(ids.len(), 2);
        assert!(
            ids.contains(&newer.id) && ids.contains(&agent.id),
            "{ids:?}"
        );
    }

    #[test]
    fn a_proposal_written_before_agents_reads_as_a_syncs() {
        let old = r#"{"id":"note-abc","note_id":"note","note_path":"01-Core/Note.md","base_hash":"h","input_hash":"abc","reason":"cited source code changed","status":"pending"}"#;
        let p: Proposal = serde_json::from_str(old).unwrap();
        assert_eq!(p.kind, ProposalKind::Sync);
        assert!(!p.pictures && p.explained.is_none() && p.model.is_none());
    }

    #[test]
    fn rejects_path_traversal_ids() {
        let dir = vault_with_note();
        assert!(get(dir.path(), "../../secret").is_err());
        assert!(proposed_text(dir.path(), "..\\x").is_err());
    }
}
