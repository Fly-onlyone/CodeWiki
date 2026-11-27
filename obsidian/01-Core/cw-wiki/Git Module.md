---
tags: [core, cw-wiki, module]
---

# Git Module

> Minimal git access through the `git` command line.

## What it does

A few small questions about git are needed in many places: what is HEAD, has the code moved since the wiki was written, and is this folder a linked worktree. This module answers them by running the `git` command line. It does not link a git library. Every failure becomes `None` or `false`, so callers never have to handle a git error.

## Source

- `crates/cw-wiki/src/git.rs` — module `cw_wiki::git`

## How it works

A private helper `run` calls `git -C <root> <args>` and returns trimmed output only when git exits with success.

`head_commit` runs `rev-parse --verify HEAD`. `changed_files` runs `diff --name-only base..HEAD`. `vault_is_behind` compares the commit the wiki was generated at with HEAD. If they are equal the wiki is current. If not, it lists the changed files and says behind only when at least one lies outside the vault folder. If the old commit is unknown (rebased away, shallow clone) the wiki counts as behind.

`is_linked_worktree` asks git for the git directory and the common directory and says true when they differ after resolving the paths.

## Items

- `fn raw(root: &Path, args: &[&str]) -> Option<String>` — standard output of a git command as it was printed. `None` when git is missing, `root` is no repository or the command fails.
- `fn entries(out: &str) -> impl Iterator<Item = &str>` — the entries of an output asked for with `-z`.
- `pub fn history_paths(root: &Path) -> HashSet<String>` — every path under `root` that a commit of any branch ever held, deleted ones included. Empty outside a repository.
- `pub fn moves(root: &Path) -> HashMap<String, String>` — where each path that git saw renamed is now: the old path, and the last path of its chain of renames. Empty outside a repository.
- `pub fn at_head(root: &Path) -> Option<HashSet<String>>` — paths under `root` in the commit at `HEAD`. `None` outside a repository or before the first commit.
- `pub fn changed(root: &Path) -> Option<HashSet<String>>` — paths under `root` with a change that is not committed: edited, added or deleted, staged or not. `None` outside a repository.
- `pub fn recoverable(root: &Path) -> HashSet<String>` — Vault-relative paths of the notes git can bring back as they are now: tracked, and without a change since their last commit, staged or not. Empty outside a repository.
- `fn run(root: &Path, args: &[&str]) -> Option<String>` — private function, returns `Option<String>`; called by `changed`, `changed_files`, `head_commit`, `is_linked_worktree` and 2 more.
- `pub fn head_commit(root: &Path) -> Option<String>` — commit id of `HEAD`, or `None` outside a repository or before the first commit.
- `pub fn vault_is_behind(root: &Path, generated: Option<&str>) -> bool` — true when the code moved on since the vault was generated at commit `generated`. Commits that touch only the vault do not count: committing the wiki must not make the wiki look out of date.
- `pub fn changed_files(root: &Path, base: &str) -> Option<Vec<String>>` — Repo-relative paths changed between `base` and `HEAD`. `None` when `base` is unknown to the repository (rebased away, shallow clone).
- `pub fn is_linked_worktree(root: &Path) -> bool` — true when `root` is a linked worktree: its git directory is not the repository's common one. Any failure counts as false. Git 2.31 and later print both directories as absolute paths when asked with `--path-format=absolute`. An older git echoes the flag it does not know as a line of its own and prints paths relative to `root`, so that line is skipped and a relative path is read against `root`.
- `pub fn main_checkout(root: &Path) -> Option<std::path::PathBuf>` — the main checkout of the repository that the linked worktree `root` belongs to: the folder that holds the repository's common git directory. `None` when `root` is no linked worktree.

## Depends on

- [[cw-core]] — uses `VAULT_DIR`

## Used by

- [[cw-wiki]] — from `a_deletion_that_is_not_committed_is_still_at_head`, `a_linked_worktree_is_told_from_the_main_checkout`, `a_repository_inside_a_bigger_checkout_reads_its_own_paths`, `history_keeps_deleted_paths_and_follows_moves_to_their_end`
- [[Git]] — from `changed`, `head`, `history`, `moves`
- [[Service Module]] — from `is_stale`, `reindex`, `run_update`
- [[codewiki]] — from `session_start_hook`
- [[Repos Module]] — from `main_of`
- [[Briefing Module]] — from `session_start`
- [[Explanations Module]] — from `submit`
- [[Query Module]] — from `status`

## Gotchas

- Commits that touch only the vault never make the wiki look behind. Committing the wiki must not make it stale.
- Without a HEAD (no repository, or no commit yet) the answers are `None`, and `vault_is_behind` compares the two optional values directly.
- Git before 2.31 does not know `--path-format=absolute`. It echoes the flag as an output line and prints relative paths. The code skips that line and reads relative paths against the root.
- A missing `git` program is silent: everything reads as no repository.

## See also

- [[_index]]
- [[cw-wiki]]
