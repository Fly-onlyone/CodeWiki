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

/// The agent host a briefing is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    ClaudeCode,
    Codex,
}

/// What the line about waiting notes asks of the agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Explain {
    /// A fact only.
    Facts,
    /// Launch the plugin agent in the background.
    ClaudeCode,
    /// Use the `write-notes` skill after the first request.
    Codex,
}

/// Whether the waiting-notes line orders a batch: only when auto explaining is
/// on, the session starts fresh (`startup`, `resume`, `clear`), it is not in
/// plan mode and not in a linked worktree.
pub fn explain_for(
    host: Host,
    auto_on: bool,
    source: Option<&str>,
    plan: bool,
    worktree: bool,
) -> Explain {
    let fresh = matches!(source, Some("startup" | "resume" | "clear"));
    if !auto_on || !fresh || plan || worktree {
        return Explain::Facts;
    }
    match host {
        Host::ClaudeCode => Explain::ClaudeCode,
        Host::Codex => Explain::Codex,
    }
}

fn explain_sentence(explain: Explain) -> String {
    let max = crate::explanations::MAX_LIMIT;
    match explain {
        Explain::Facts => {
            "This line says what waits; write notes when the user asks, or for files you changed."
                .to_string()
        }
        Explain::ClaudeCode => format!(
            "The user turned on auto explaining: when the user sends their first message, unless you are in plan mode, launch the `codewiki:note-writer` agent in the background to explain up to {max} waiting notes, stale first, tell the user in one line that it is writing wiki notes, then go on with their request. Do not write these notes yourself."
        ),
        Explain::Codex => format!(
            "The user turned on auto explaining: after you finish the user's first request, unless you are in plan mode, explain up to {max} waiting notes, stale first, with the `write-notes` skill, and tell the user in one line how many you wrote."
        ),
    }
}

/// Plain-text briefing for an agent starting work in the repository at
/// `root`: whether a wiki exists, where its home note is, whether it is
/// behind `HEAD`, how many proposals wait for review, how many notes wait
/// for an explanation, and which tools read it.
///
/// The lines state facts; an agent host adds them to the session as context.
/// The line about waiting notes is a fact too, and says so: an agent that
/// took it as an order would spend its user's allowance on notes at the
/// start of every session. It becomes an order for a bounded batch only when
/// the user turned auto explaining on (`cw_core::auto_explain`), the session
/// starts fresh (startup, resume, clear), is not in plan mode and is not in a
/// linked worktree; `explain` says which, see [`explain_for`].
pub fn session_start(root: &Path, explain: Explain) -> Result<String> {
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
        (at, Some(_)) if !crate::git::vault_is_behind(root, at) => {
            out.push_str("The wiki is up to date with the code at HEAD.\n");
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
    let waiting = crate::explanations::waiting_counts(root)?;
    if waiting.total() > 0 {
        out.push_str(&format!(
            "Notes waiting for an explanation: {} ({} whose explanation may be behind the code, {} never explained). CodeWiki writes each note's facts from the code; what the thing is for and how it works is written by a coding agent, with the MCP tools `list_waiting_notes`, `get_note_brief` and `submit_note`. {}\n",
            waiting.total(),
            waiting.behind,
            waiting.unexplained,
            explain_sentence(explain)
        ));
    }
    out.push_str(
        "The CodeWiki MCP tools `get_overview` (architecture and index status) and `get_context` (what to know before touching a file or symbol) answer from this wiki. Call them before exploring the code by hand.\n",
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: Option<&str> = Some("startup");

    #[test]
    fn fresh_sessions_get_the_order_of_their_host() {
        for source in ["startup", "resume", "clear"] {
            assert_eq!(
                explain_for(Host::ClaudeCode, true, Some(source), false, false),
                Explain::ClaudeCode
            );
            assert_eq!(
                explain_for(Host::Codex, true, Some(source), false, false),
                Explain::Codex
            );
        }
    }

    #[test]
    fn everything_else_gets_the_facts() {
        for source in [Some("compact"), Some("fork"), Some("other"), None] {
            assert_eq!(
                explain_for(Host::ClaudeCode, true, source, false, false),
                Explain::Facts,
                "{source:?}"
            );
        }
        assert_eq!(
            explain_for(Host::Codex, true, START, true, false),
            Explain::Facts
        );
        assert_eq!(
            explain_for(Host::Codex, true, START, false, true),
            Explain::Facts
        );
        assert_eq!(
            explain_for(Host::ClaudeCode, false, START, false, false),
            Explain::Facts
        );
    }
}
