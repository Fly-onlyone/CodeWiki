---
tags: [core, cw-server, module]
---

# Repos Module

> The list of repositories this backend manages, stored as `repos.json`.

## What it does

Stores the list of repositories the backend manages, as `repos.json` in the data folder. Each entry has an id, a display name, the absolute path on the backend's machine and, for a linked git worktree, the id of its main checkout (`worktree_of`). The API, the git webhook, the workspace view and the embedding job all read it. A worktree is one more copy of a repository, often made by a coding agent for one task, so it is not treated as a project of its own.

## Source

- `crates/cw-server/src/repos.rs` — module `cw_server::repos`

## How it works

`load` reads and parses the file. A missing file gives an empty list. `add` turns the path into a canonical folder path, checks that it is a folder, and builds the id with `repo_id`. If that id is known, it returns the old entry. Otherwise it adds an entry named after the folder, asks git through `main_of` whether the folder is a linked worktree, sorts by lowercase name and saves. `remove` drops an entry by id and fails with not found if there is none. `get` finds one entry. `save` writes pretty JSON with a final newline.

Two functions keep worktrees tidy. `mark_worktrees` marks entries that were registered before worktrees were told apart. It asks git once for each unmarked entry whose folder exists, so it runs at backend start, not on each listing. `prune_worktrees` forgets every worktree whose folder is gone. It deletes the index database and its `-wal` and `-shm` files first, and only drops the entry when the database is really gone.

## Items

- `pub struct RepoEntry` — public struct; used by `ListedRepo`, `add`, `add_repo`, `embed_all` and 9 more.
- `fn main_of(root: &Path) -> Option<String>` — the id of the main checkout `root` is a linked worktree of, if it is one.
- `fn file(data_dir: &Path) -> PathBuf` — private function, returns `PathBuf`; called by `load`, `save`.
- `pub fn load(data_dir: &Path) -> Result<Vec<RepoEntry>>` — public function, returns `Result<Vec<RepoEntry>>`; called by `add`, `embed_all`, `get`, `git_hook` and 8 more.
- `fn save(data_dir: &Path, repos: &[RepoEntry]) -> Result<()>` — private function, returns `Result<()>`; called by `add`, `mark_worktrees`, `prune_worktrees`, `remove`.
- `pub fn add(data_dir: &Path, path: &str) -> Result<RepoEntry>` — add a repository by path. Adding the same path twice returns the existing entry.
- `pub fn mark_worktrees(data_dir: &Path) -> Result<()>` — mark the entries that are linked worktrees and were registered before worktrees were told apart. Asks git once for each unmarked entry whose folder exists, so it runs at the backend's start, not on each listing.
- `pub fn prune_worktrees(data_dir: &Path) -> Result<Vec<RepoEntry>>` — forget every linked worktree whose folder is gone, with its index: the folder of a worktree does not come back, unlike a repository on a drive that is unplugged for a while. Returns the entries forgotten.
- `pub fn remove(data_dir: &Path, id: &str) -> Result<()>` — forget a repository. Its vault and source are not touched.
- `pub fn get(data_dir: &Path, id: &str) -> Result<RepoEntry>` — public function, returns `Result<RepoEntry>`; called by `Repo::open`, `get_repo`, `repo_contracts`.

## Depends on

- [[Error]] — uses `external`, `invalid`, `io`, `not_found`
- [[Error Module]] — uses `Result`
- [[Paths Module]] — uses `repo_id`
- [[Git Module]] — uses `main_checkout`

## Used by

- [[Api Module]] — from `ListedRepo`, `add_repo`, `get_repo`, `git_hook`
- [[Workspace Module (cw-server)]] — from `open`, `repo_contracts`, `summarize`, `workspace`
- [[Embed Job Module]] — from `embed_all`, `embed_repo`, `resume`
- [[cw-server]] — from `serve`

## Gotchas

- Callers that change the list must hold the write lock. The module itself does no locking, and `save` rewrites the whole file.
- `remove` forgets the entry only. The vault and the source stay. `prune_worktrees` is different: it also deletes the index of the worktree.
- On Windows an open file cannot be deleted. If a job still holds the index of a gone worktree, the entry stays listed and the next call tries again.
- Only worktrees are pruned. A normal repository on an unplugged drive stays in the list.
- On Windows the `\\?\` prefix of canonical paths is cut off, because tools reject it.
- A corrupt `repos.json` makes `load` fail with an error. Only a missing file gives an empty list.
- The same folder added by a different path spelling gives the same id and the same entry.

## See also

- [[_index]]
- [[cw-server]]
