---
tags: [core, cw-wiki, module]
---

# Briefing Module

> The briefing a coding agent is given when it starts a session.

## What it does

When a coding agent starts a session in a repository, this module builds the short plain-text briefing it is given. The briefing says whether a wiki exists, whether it is stale, how many proposals and unexplained notes wait, and which CodeWiki tools to call first. It reads only the vault manifest and one git call, never the index, so a session start never waits on a scan or a backend.

## Source

- `crates/cw-wiki/src/briefing.rs` — module `cw_wiki::briefing`

## How it works

`vault_root` walks up from the start folder to find the nearest folder that holds a vault, so a session opened in a subfolder still finds the wiki.

`session_start` then builds the text:

- If the vault has no notes or no home note, it says no wiki exists and points to the `sync_wiki` tool.
- Otherwise it states the note count and the home note.
- It compares the manifest's last indexed commit with git HEAD (`head_commit`, `vault_is_behind`). The text says: no HEAD, up to date, or stale with both short commit ids.
- It counts pending proposals.
- It adds a line about waiting notes when any wait, using `waiting_counts`.
- It ends with a pointer to `get_overview` and `get_context`.

The waiting-notes line ends with a sentence chosen by `explain_sentence`. `explain_for` returns an order (`ClaudeCode` or `Codex`, each with a [[Count]]) only when auto explaining is on, the session source is `startup`, `resume` or `clear`, and the session is neither in plan mode nor in a linked worktree. In every other case it returns `Facts`, a sentence that only states what waits.

For Claude Code the order tells the agent to launch the `codewiki:note-writer` agent in the background after the user's first message. `launch_order` caps one agent at `MAX_LIMIT` (20) notes, so a larger count launches several agents at once, each told how many notes to take. For Codex the order tells the agent to use the `write-notes` skill after the first request, 20 notes at a time. The count is capped by how many notes wait.

## Items

- `const HOME_NOTE: &str` — name of the vault's home note.
- `pub fn vault_root(start: &Path) -> Option<PathBuf>` — the nearest folder at or above `start` that holds a vault, so a session started in a subfolder still finds the repository's wiki.
- `fn short(commit: &str) -> &str` — private function, returns `&str`; called by `session_start`.
- `pub enum Host` — the agent host a briefing is for.
- `pub enum Explain` — what the line about waiting notes asks of the agent.
- `explain_for` — whether the waiting-notes line orders a batch: only when auto explaining is on, the session starts fresh (`startup`, `resume`, `clear`), it is not in plan mode and not in a linked worktree. `count` is how many notes the user chose (`cw_core::auto_explain::load_count`).
- `fn launch_order(notes: usize) -> String` — how many plugin agents explain `notes` notes, and what each is told: an agent is lent at most one queue's worth, `MAX_LIMIT` notes, so a larger count takes several agents, all launched at once. `MAX_LIMIT`: crate::explanations::MAX_LIMIT.
- `fn explain_sentence(explain: Explain, waiting: usize) -> String` — the order or the fact that ends the line about the `waiting` notes.
- `pub fn session_start(root: &Path, explain: Explain) -> Result<String>` — Plain-text briefing for an agent starting work in the repository at `root`: whether a wiki exists, where its home note is, whether it is behind `HEAD`, how many proposals wait for review, how many notes wait for an explanation, and which tools read it. The lines state facts; an agent host adds them to the session as context. The line about waiting notes is a fact too, and says so: an agent that took it as an order would spend its user's allowance on notes at the start of every session. It becomes an order for the batch the user chose only when the user turned auto explaining on (`cw_core::auto_explain`), the session starts fresh (startup, resume, clear), is not in plan mode and is not in a linked worktree; `explain` says which, see `explain_for`.

## Depends on

- [[cw-core]] — uses `MANIFEST_DIR`, `VAULT_DIR`
- [[Explanations Module]] — uses `MAX_LIMIT`, `waiting_counts`
- [[Git Module]] — uses `head_commit`, `vault_is_behind`
- [[VaultMeta]] — uses `live_notes`, `load`
- [[Error Module]] — uses `Result`
- [[Proposal Module]] — uses `list`
- [[Count]] — uses `Count`
- [[Counts]] — uses `total`

## Used by

- [[cw-wiki]] — from `a_folder_without_git_is_not_called_stale`, `a_note_that_waits_to_be_retired_is_no_note_of_the_wiki`, `reports_a_missing_wiki_then_its_state`, `the_line_about_waiting_notes_goes_when_none_waits`
- [[codewiki]] — from `session_start_hook`

## Gotchas

- The order to write notes is deliberately opt-in. Without it an agent might spend the user's usage allowance on notes at every session start.
- Sessions with source `compact` or `fork` get facts only, never an order.
- Linked worktrees never get the order, so parallel sessions do not all write notes at once. The caller in the hook checks the worktree and downgrades the order to facts.
- A count above 20 launches several agents in one message. A count of all with many waiting notes can launch a dozen agents.
- The commit is shortened to 10 characters, and a missing manifest commit shows as unknown.
- `HOME_NOTE` is fixed as `_HOME.md`; a vault without that file counts as having no wiki.

## See also

- [[_index]]
- [[cw-wiki]]
