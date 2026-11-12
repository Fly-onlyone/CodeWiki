---
tags: [core, cw-wiki, module]
---

# Briefing Module

> The briefing a coding agent is given when it starts a session.

## Source

- `crates/cw-wiki/src/briefing.rs` — module `cw_wiki::briefing`

## How it works

Main items:

- `session_start` — Plain-text briefing for an agent starting work in the repository at `root`: whether a wiki exists, where its home note is, whether it is behind `HEAD`, how many proposals wait for review, and which tools read it.
- `vault_root` — the nearest folder at or above `start` that holds a vault, so a session started in a subfolder still finds the repository's wiki.
- `short` — function
- `HOME_NOTE` — name of the vault's home note.

## Depends on

- [[cw-core]] — uses `MANIFEST_DIR`, `VAULT_DIR`
- [[Error Module]] — uses `Result`
- [[Proposal Module]] — uses `list`
- [[Git Module]] — uses `head_commit`
- [[VaultMeta]] — uses `load`

## Used by

- [[cw-wiki]] — from `a_folder_without_git_is_not_called_stale`, `reports_a_missing_wiki_then_its_state`
- [[codewiki]] — from `session_start_hook`

## See also

- [[_index]]
- [[cw-wiki]]
