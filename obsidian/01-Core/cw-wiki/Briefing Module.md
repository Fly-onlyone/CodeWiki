---
tags: [core, cw-wiki, module]
---

# Briefing Module

> The briefing a coding agent is given when it starts a session.

## What it does

It is read from the vault's manifest and one `git` call, never from the index: a session start must not wait on a scan or a backend.

## Source

- `crates/cw-wiki/src/briefing.rs` — module `cw_wiki::briefing`

## How it works

1. `session_start_hook` in [[codewiki]] calls `vault_root`.
2. `session_start_hook` in [[codewiki]] calls `explain_for`.
3. `session_start_hook` in [[codewiki]] calls `session_start`. `session_start` calls `VaultMeta::load` in [[VaultMeta]], then `head_commit`, then `vault_is_behind` (both in [[Git Module]]), then `short`, then `list` in [[Proposal Module]], then `waiting_counts` in [[Explanations Module]], and 1 more.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const HOME_NOTE: &str` — name of the vault's home note.
- `pub fn vault_root(start: &Path) -> Option<PathBuf>` — the nearest folder at or above `start` that holds a vault, so a session started in a subfolder still finds the repository's wiki.
- `fn short(commit: &str) -> &str` — private function, returns `&str`; called by `session_start`.
- `pub enum Host` — the agent host a briefing is for.
- `pub enum Explain` — what the line about waiting notes asks of the agent.
- `explain_for` — whether the waiting-notes line orders a batch: only when auto explaining is on, the session starts fresh (`startup`, `resume`, `clear`), it is not in plan mode and not in a linked worktree.
- `pub fn session_start(root: &Path, explain: Explain) -> Result<String>` — Plain-text briefing for an agent starting work in the repository at `root`: whether a wiki exists, where its home note is, whether it is behind `HEAD`, how many proposals wait for review, how many notes wait for an explanation, and which tools read it. The lines state facts; an agent host adds them to the session as context. The line about waiting notes is a fact too, and says so: an agent that took it as an order would spend its user's allowance on notes at the start of every session. It becomes an order for a bounded batch only when the user turned auto explaining on (`cw_core::auto_explain`), the session starts fresh (startup, resume, clear), is not in plan mode and is not in a linked worktree; `explain` says which, see `explain_for`.

And 1 more private item with no comment.

## Depends on

- [[cw-core]] — uses `MANIFEST_DIR`, `VAULT_DIR`
- [[Explanations Module]] — uses `MAX_LIMIT`, `waiting_counts`
- [[Git Module]] — uses `head_commit`, `vault_is_behind`
- [[Error Module]] — uses `Result`
- [[Proposal Module]] — uses `list`
- [[VaultMeta]] — uses `load`
- [[Counts]] — uses `total`

## Used by

- [[cw-wiki]] — from `a_folder_without_git_is_not_called_stale`, `reports_a_missing_wiki_then_its_state`, `the_line_about_waiting_notes_goes_when_none_waits`, `the_waiting_line_orders_a_batch_for_each_host`
- [[codewiki]] — from `session_start_hook`

## See also

- [[_index]]
- [[cw-wiki]]
