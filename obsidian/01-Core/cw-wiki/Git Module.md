---
tags: [core, cw-wiki, module]
---

# Git Module

> Minimal git access through the `git` command line.

## Source

- `crates/cw-wiki/src/git.rs` — module `cw_wiki::git`

## How it works

Main items:

- `head_commit` — commit id of `HEAD`, or `None` outside a repository or before the first commit.
- `vault_is_behind` — true when the code moved on since the vault was generated at commit `generated`.
- `changed_files` — Repo-relative paths changed between `base` and `HEAD`.
- `run` — function

## Depends on

- [[cw-core]] — uses `VAULT_DIR`

## Used by

- [[Service Module]] — from `is_stale`, `reindex`, `update_in`
- [[cw-wiki]] — from `update`
- [[Briefing Module]] — from `session_start`
- [[Query Module]] — from `status`

## See also

- [[_index]]
- [[cw-wiki]]
