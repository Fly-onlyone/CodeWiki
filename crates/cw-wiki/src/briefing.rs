//! The briefing a coding agent is given when it starts a session.
//!
//! It is read from the vault's manifest and one `git` call, never from the
//! index: a session start must not wait on a scan or a backend.

use std::path::{Path, PathBuf};

use cw_core::{MANIFEST_DIR, Result, VAULT_DIR};
use cw_vault::{VaultMeta, proposal};

use crate::git::head_commit;

/// Name of the vault's home note.
const HOME_NOTE: &str = "_HOME.md";

/// The nearest folder at or above `start` that holds a vault, so a session
/// started in a subfolder still finds the repository's wiki.
pub fn vault_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| dir.join(VAULT_DIR).join(MANIFEST_DIR).is_dir())
        .map(Path::to_path_buf)
}

fn short(commit: &str) -> &str {
    commit.get(..10).unwrap_or(commit)
}

/// Plain-text briefing for an agent starting work in the repository at
/// `root`: whether a wiki exists, where its home note is, whether it is
/// behind `HEAD`, how many proposals wait for review, and which tools read it.
///
/// The lines state facts; an agent host adds them to the session as context.
pub fn session_start(root: &Path) -> Result<String> {
    let vault = root.join(VAULT_DIR);
    let meta = VaultMeta::load(&vault)?;
    let home = vault.join(HOME_NOTE);
    if meta.notes.is_empty() || !home.is_file() {
        return Ok(format!(
            "CodeWiki: no wiki has been generated for this repository yet (its home note would be `{VAULT_DIR}/{HOME_NOTE}`).\n\
             The CodeWiki MCP tool `sync_wiki` generates it. After that, `get_overview` and `get_context` answer questions about architecture and about single files or symbols.\n"
        ));
    }
    let mut out = format!(
        "CodeWiki: this repository has a maintained wiki of {} notes in `{VAULT_DIR}/`. Its home note is `{VAULT_DIR}/{HOME_NOTE}`.\n",
        meta.notes.len()
    );
    let generated = meta.manifest.last_indexed_commit.as_deref();
    match (generated, head_commit(root)) {
        (_, None) => {
            out.push_str("This folder has no git HEAD, so the wiki cannot be compared with it.\n")
        }
        (Some(at), Some(head)) if at == head => {
            out.push_str("The wiki is up to date with HEAD.\n");
        }
        (at, Some(head)) => out.push_str(&format!(
            "The wiki is stale: it was generated at commit {}, and HEAD is {}. The MCP tool `sync_wiki` brings it up to date.\n",
            at.map_or("unknown", short),
            short(&head)
        )),
    }
    let pending = proposal::list(&vault)?
        .iter()
        .filter(|p| p.status == proposal::ProposalStatus::Pending)
        .count();
    out.push_str(&format!(
        "Pending proposals: {pending}{}.\n",
        if pending == 0 {
            ""
        } else {
            " (changes to human-edited notes that wait for review; those notes may lag behind the code)"
        }
    ));
    out.push_str(
        "The CodeWiki MCP tools `get_overview` (architecture and index status) and `get_context` (what to know before touching a file or symbol) answer from this wiki. Call them before exploring the code by hand.\n",
    );
    Ok(out)
}
