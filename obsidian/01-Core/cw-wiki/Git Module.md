---
tags: [core, cw-wiki, module]
---

# Git Module

> Minimal git access through the `git` command line.

## Source

- `crates/cw-wiki/src/git.rs` — module `cw_wiki::git`

## How it works

1. `session_start` in [[Briefing Module]] and `submit` in [[Explanations Module]], and 3 more, call `head_commit`. `head_commit` calls `run`.
2. `session_start` in [[Briefing Module]] and `status` in [[Query Module]], and 1 more, call `vault_is_behind`. `vault_is_behind` calls `head_commit`, then `changed_files`. `changed_files` calls `run`.
3. `session_start_hook` in [[codewiki]] calls `is_linked_worktree`. `is_linked_worktree` calls `run`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `fn run(root: &Path, args: &[&str]) -> Option<String>` — private function, returns `Option<String>`; called by `changed_files`, `head_commit`, `is_linked_worktree`.
- `pub fn head_commit(root: &Path) -> Option<String>` — commit id of `HEAD`, or `None` outside a repository or before the first commit.
- `pub fn vault_is_behind(root: &Path, generated: Option<&str>) -> bool` — true when the code moved on since the vault was generated at commit `generated`. Commits that touch only the vault do not count: committing the wiki must not make the wiki look out of date.
- `pub fn changed_files(root: &Path, base: &str) -> Option<Vec<String>>` — Repo-relative paths changed between `base` and `HEAD`. `None` when `base` is unknown to the repository (rebased away, shallow clone).
- `pub fn is_linked_worktree(root: &Path) -> bool` — true when `root` is a linked worktree: its git directory is not the repository's common one. Any failure counts as false. Git 2.31 and later print both directories as absolute paths when asked with `--path-format=absolute`. An older git echoes the flag it does not know as a line of its own and prints paths relative to `root`, so that line is skipped and a relative path is read against `root`.

## Depends on

- [[cw-core]] — uses `VAULT_DIR`

## Used by

- [[Service Module]] — from `is_stale`, `reindex`, `run_update`
- [[cw-wiki]] — from `a_linked_worktree_is_told_from_the_main_checkout`, `update`
- [[codewiki]] — from `session_start_hook`
- [[Briefing Module]] — from `session_start`
- [[Explanations Module]] — from `submit`
- [[Query Module]] — from `status`

## See also

- [[_index]]
- [[cw-wiki]]
