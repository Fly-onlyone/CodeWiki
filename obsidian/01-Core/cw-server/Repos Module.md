---
tags: [core, cw-server, module]
---

# Repos Module

> The list of repositories this backend manages, stored as `repos.json`.

## Source

- `crates/cw-server/src/repos.rs` — module `cw_server::repos`

## How it works

1. `git_hook` and `list_repos` in [[Api Module]], and 4 more, call `load`. `load` calls `file`, then `Error::external`, then `Error::io` (both in [[Error]]).
2. `add_repo` in [[Api Module]] calls `add`. `add` calls `Error::invalid` in [[Error]], then `load`, then `repo_id` in [[Paths Module]], then `save`. `save` calls `Error::io`, then `Error::external` (both in [[Error]]), then `file`.
3. `remove_repo` in [[Api Module]] calls `remove`. `remove` calls `load`, then `Error::not_found` in [[Error]], then `save`.
4. `Repo::open` and `get_repo` in [[Api Module]], and 1 more, call `get`. `get` calls `load`, then `Error::not_found` in [[Error]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub struct RepoEntry` — public struct; used by `ListedRepo`, `add`, `add_repo`, `embed_all` and 8 more.
- `fn file(data_dir: &Path) -> PathBuf` — private function, returns `PathBuf`; called by `load`, `save`.
- `pub fn load(data_dir: &Path) -> Result<Vec<RepoEntry>>` — public function, returns `Result<Vec<RepoEntry>>`; called by `add`, `embed_all`, `get`, `git_hook` and 5 more.
- `fn save(data_dir: &Path, repos: &[RepoEntry]) -> Result<()>` — private function, returns `Result<()>`; called by `add`, `remove`.
- `pub fn add(data_dir: &Path, path: &str) -> Result<RepoEntry>` — add a repository by path. Adding the same path twice returns the existing entry.
- `pub fn remove(data_dir: &Path, id: &str) -> Result<()>` — forget a repository. Its vault and source are not touched.
- `pub fn get(data_dir: &Path, id: &str) -> Result<RepoEntry>` — public function, returns `Result<RepoEntry>`; called by `Repo::open`, `get_repo`, `repo_contracts`.

## Depends on

- [[Error]] — uses `external`, `invalid`, `io`, `not_found`
- [[Error Module]] — uses `Result`
- [[Paths Module]] — uses `repo_id`

## Used by

- [[Api Module]] — from `ListedRepo`, `add_repo`, `get_repo`, `git_hook`
- [[Workspace Module (cw-server)]] — from `open`, `repo_contracts`, `summarize`, `workspace`
- [[Embed Job Module]] — from `embed_all`, `embed_repo`, `resume`

## See also

- [[_index]]
- [[cw-server]]
