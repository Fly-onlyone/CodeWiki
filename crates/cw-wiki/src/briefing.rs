//! The briefing a coding agent is given when it starts a session.
//!
//! It is read from the vault's manifest and one `git` call, never from the
//! index: a session start must not wait on a scan or a backend.

use std::path::{Path, PathBuf};

use cw_core::auto_explain::Count;
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
    /// Launch the plugin agent in the background, one for each batch of
    /// notes, as many notes as the count says.
    ClaudeCode(Count),
    /// Use the `write-notes` skill after the first request, as many notes as
    /// the count says.
    Codex(Count),
}

/// Whether the waiting-notes line orders a batch: only when auto explaining is
/// on, the session starts fresh (`startup`, `resume`, `clear`), it is not in
/// plan mode and not in a linked worktree. `count` is how many notes the user
/// chose (`cw_core::auto_explain::load_count`).
pub fn explain_for(
    host: Host,
    auto_on: bool,
    count: Count,
    source: Option<&str>,
    plan: bool,
    worktree: bool,
) -> Explain {
    let fresh = matches!(source, Some("startup" | "resume" | "clear"));
    if !auto_on || !fresh || plan || worktree {
        return Explain::Facts;
    }
    match host {
        Host::ClaudeCode => Explain::ClaudeCode(count),
        Host::Codex => Explain::Codex(count),
    }
}

/// How many plugin agents explain `notes` notes, and what each is told: an
/// agent is lent at most one queue's worth, [`MAX_LIMIT`] notes, so a larger
/// count takes several agents, all launched at once.
///
/// [`MAX_LIMIT`]: crate::explanations::MAX_LIMIT
fn launch_order(notes: usize) -> String {
    let batch = crate::explanations::MAX_LIMIT;
    let agents = notes.div_ceil(batch);
    if agents <= 1 {
        return format!(
            "launch the `codewiki:note-writer` agent in the background to explain up to {notes} waiting notes, stale first, tell the user in one line that it is writing wiki notes"
        );
    }
    let (full, rest) = (notes / batch, notes % batch);
    let share = if rest == 0 {
        format!("tell each to explain {batch} waiting notes")
    } else if full == 1 {
        format!("tell one to explain {batch} waiting notes and the other to explain {rest}")
    } else {
        format!(
            "tell {full} of them to explain {batch} waiting notes each and the last to explain {rest}"
        )
    };
    format!(
        "launch {agents} `codewiki:note-writer` agents in the background at once, in one message, and {share}, stale first; tell the user in one line that {agents} agents are writing wiki notes"
    )
}

/// The order or the fact that ends the line about the `waiting` notes.
fn explain_sentence(explain: Explain, waiting: usize) -> String {
    match explain {
        Explain::Facts => {
            "This line says what waits; write notes when the user asks, or for files you changed."
                .to_string()
        }
        Explain::ClaudeCode(count) => format!(
            "The user turned on auto explaining: when the user sends their first message, unless you are in plan mode, {}, then go on with their request. Do not write these notes yourself.",
            launch_order(count.of(waiting))
        ),
        Explain::Codex(count) => {
            let batch = crate::explanations::MAX_LIMIT;
            let notes = match count {
                Count::All => "every waiting note".to_string(),
                Count::Notes(_) => format!("up to {} waiting notes", count.of(waiting)),
            };
            format!(
                "The user turned on auto explaining: after you finish the user's first request, unless you are in plan mode, explain {notes}, stale first, with the `write-notes` skill, listing {batch} at a time, and tell the user in one line how many you wrote."
            )
        }
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
/// start of every session. It becomes an order for the batch the user chose
/// only when the user turned auto explaining on (`cw_core::auto_explain`), the session
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
            explain_sentence(explain, waiting.total())
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
    const FIFTY: Count = Count::Notes(50);

    #[test]
    fn fresh_sessions_get_the_order_of_their_host() {
        for source in ["startup", "resume", "clear"] {
            assert_eq!(
                explain_for(Host::ClaudeCode, true, FIFTY, Some(source), false, false),
                Explain::ClaudeCode(FIFTY)
            );
            assert_eq!(
                explain_for(Host::Codex, true, Count::All, Some(source), false, false),
                Explain::Codex(Count::All)
            );
        }
    }

    #[test]
    fn everything_else_gets_the_facts() {
        for source in [Some("compact"), Some("fork"), Some("other"), None] {
            assert_eq!(
                explain_for(Host::ClaudeCode, true, FIFTY, source, false, false),
                Explain::Facts,
                "{source:?}"
            );
        }
        assert_eq!(
            explain_for(Host::Codex, true, FIFTY, START, true, false),
            Explain::Facts
        );
        assert_eq!(
            explain_for(Host::Codex, true, FIFTY, START, false, true),
            Explain::Facts
        );
        assert_eq!(
            explain_for(Host::ClaudeCode, false, FIFTY, START, false, false),
            Explain::Facts
        );
    }

    #[test]
    fn a_batch_that_fits_one_agent_launches_one() {
        let text = explain_sentence(Explain::ClaudeCode(Count::Notes(20)), 7);
        assert!(
            text.contains("launch the `codewiki:note-writer` agent in the background to explain up to 7 waiting notes"),
            "{text}"
        );
        let text = explain_sentence(Explain::ClaudeCode(Count::Notes(20)), 218);
        assert!(text.contains("up to 20 waiting notes"), "{text}");
        assert!(text.contains("plan mode") && text.contains("Do not write these notes yourself."));
    }

    #[test]
    fn a_larger_batch_launches_one_agent_for_each_twenty() {
        let text = explain_sentence(Explain::ClaudeCode(FIFTY), 218);
        assert!(
            text.contains("launch 3 `codewiki:note-writer` agents in the background at once, in one message, and tell 2 of them to explain 20 waiting notes each and the last to explain 10"),
            "{text}"
        );
        let text = explain_sentence(Explain::ClaudeCode(Count::All), 218);
        assert!(
            text.contains("launch 11 `codewiki:note-writer` agents")
                && text.contains(
                    "tell 10 of them to explain 20 waiting notes each and the last to explain 18"
                ),
            "{text}"
        );
        let text = explain_sentence(Explain::ClaudeCode(Count::Notes(100)), 218);
        assert!(
            text.contains("launch 5 `codewiki:note-writer` agents")
                && text.contains("tell each to explain 20 waiting notes"),
            "{text}"
        );
        let text = explain_sentence(Explain::ClaudeCode(Count::All), 30);
        assert!(
            text.contains("launch 2 `codewiki:note-writer` agents")
                && text
                    .contains("tell one to explain 20 waiting notes and the other to explain 10"),
            "{text}"
        );
    }

    #[test]
    fn codex_explains_the_count_itself_in_batches() {
        let text = explain_sentence(Explain::Codex(FIFTY), 218);
        assert!(
            text.contains("explain up to 50 waiting notes")
                && text.contains("listing 20 at a time"),
            "{text}"
        );
        let text = explain_sentence(Explain::Codex(Count::All), 218);
        assert!(text.contains("explain every waiting note"), "{text}");
        assert!(!text.contains("note-writer"), "{text}");
    }
}
