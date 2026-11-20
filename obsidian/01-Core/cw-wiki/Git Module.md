---
tags: [core, cw-wiki, module]
---

# Git Module

> Minimal git access through the `git` command line.

## Source

- `crates/cw-wiki/src/git.rs` — module `cw_wiki::git`

## How it works

1. `session_start` in [[Briefing Module]] and `status` in [[Query Module]], and 2 more, call `head_commit`. `head_commit` calls `run`.
2. `session_start` in [[Briefing Module]] and `status` in [[Query Module]], and 1 more, call `vault_is_behind`. `vault_is_behind` calls `head_commit`, then `changed_files`. `changed_files` calls `run`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `fn run(root: &Path, args: &[&str]) -> Option<String>` — private function, returns `Option<String>`; called by `changed_files`, `head_commit`.
- `pub fn head_commit(root: &Path) -> Option<String>` — commit id of `HEAD`, or `None` outside a repository or before the first commit.
- `pub fn vault_is_behind(root: &Path, generated: Option<&str>) -> bool` — true when the code moved on since the vault was generated at commit `generated`. Commits that touch only the vault do not count: committing the wiki must not make the wiki look out of date.
- `pub fn changed_files(root: &Path, base: &str) -> Option<Vec<String>>` — Repo-relative paths changed between `base` and `HEAD`. `None` when `base` is unknown to the repository (rebased away, shallow clone).

## Depends on

- [[cw-core]] — uses `VAULT_DIR`

## Used by

- [[Service Module]] — from `is_stale`, `reindex`, `run_update`
- [[cw-wiki]] — from `update`
- [[Briefing Module]] — from `session_start`
- [[Query Module]] — from `status`

## See also

- [[_index]]
- [[cw-wiki]]
