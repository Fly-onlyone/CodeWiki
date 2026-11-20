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
2. `session_start_hook` in [[codewiki]] calls `session_start`. `session_start` calls `VaultMeta::load` in [[VaultMeta]], then `head_commit`, then `vault_is_behind` (both in [[Git Module]]), then `short`, then `list` in [[Proposal Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const HOME_NOTE: &str` — name of the vault's home note.
- `pub fn vault_root(start: &Path) -> Option<PathBuf>` — the nearest folder at or above `start` that holds a vault, so a session started in a subfolder still finds the repository's wiki.
- `fn short(commit: &str) -> &str` — private function, returns `&str`; called by `session_start`.
- `pub fn session_start(root: &Path) -> Result<String>` — Plain-text briefing for an agent starting work in the repository at `root`: whether a wiki exists, where its home note is, whether it is behind `HEAD`, how many proposals wait for review, and which tools read it. The lines state facts; an agent host adds them to the session as context.

## Depends on

- [[cw-core]] — uses `MANIFEST_DIR`, `VAULT_DIR`
- [[Git Module]] — uses `head_commit`, `vault_is_behind`
- [[Error Module]] — uses `Result`
- [[Proposal Module]] — uses `list`
- [[VaultMeta]] — uses `load`

## Used by

- [[cw-wiki]] — from `a_folder_without_git_is_not_called_stale`, `reports_a_missing_wiki_then_its_state`
- [[codewiki]] — from `session_start_hook`

## See also

- [[_index]]
- [[cw-wiki]]
