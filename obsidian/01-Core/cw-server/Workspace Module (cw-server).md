---
tags: [core, cw-server, module]
---

# Workspace Module (cw-server)

> Views across every repository the backend manages.

## Source

- `crates/cw-server/src/workspace.rs` — module `cw_server::workspace`

## How it works

1. `run` in [[codewiki]] and `workspace_view` in [[Api Module]] call `workspace`. `workspace` calls `load` in [[Repos Module]], then `summarize`. `summarize` calls `open`, then `status` in [[Query Module]], then `hotspots`, then `load` (both in [[Gitintel Module]]).
2. `run` in [[codewiki]] and `workspace_contracts` in [[Api Module]] call `workspace_contracts`. `workspace_contracts` calls `load` in [[Repos Module]], then `repo_contracts`, then `match_workspace` in [[Contracts Module]].
3. `repo_contracts` in [[Api Module]] calls `repo_contracts`. `repo_contracts` calls `scan` in [[Contracts Module]], then `open`. `open` calls `Error::not_found` in [[Error]], then `open_index_in` in [[Service Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const TOP_HOTSPOTS: usize` — hotspots listed per repository.
- `pub struct WorkspaceRepo` — one repository in the workspace view.
- `pub struct SharedDependency` — an external package more than one repository uses.
- `pub struct Workspace` — public struct; used by `workspace`, `workspace_view`.
- `pub struct RepoError` — A repository left out of a workspace result, and why.
- `pub struct WorkspaceContractsView` — contracts matched across the workspace.
- `fn open(data_dir: &Path, entry: &RepoEntry) -> Result<cw_index::Index>` — the index of a registered repository, or a clear error when its folder is gone.
- `fn summarize(data_dir: &Path, entry: &RepoEntry) -> Result<(WorkspaceRepo, BTreeSet<String>)>` — counts, hotspots and external packages of one repository.
- `pub fn workspace(data_dir: &Path) -> Result<Workspace>` — one entry per registered repository, and the external packages two or more of them use. A repository that cannot be read is listed with its error instead of failing the call.
- `pub fn repo_contracts(data_dir: &Path, entry: &RepoEntry) -> Result<RepoContracts>` — HTTP contracts of one registered repository.
- `pub fn workspace_contracts(data_dir: &Path) -> Result<WorkspaceContractsView>` — contracts of every registered repository, matched against each other.

## Depends on

- [[Contracts Module]] — uses `RepoContracts`, `WorkspaceContracts`, `match_workspace`, `scan`
- [[Gitintel Module]] — uses `Hotspot`, `hotspots`, `load`
- [[Index]] — uses `Index`, `crates`, `external_uses`
- [[Repos Module]] — uses `RepoEntry`, `load`
- [[Error Module]] — uses `Result`
- [[Query Module]] — uses `status`
- [[Service Module]] — uses `open_index_in`
- [[Error]] — uses `not_found`

## Used by

- [[Api Module]] — from `repo_contracts`, `workspace_contracts`, `workspace_view`
- [[codewiki]] — from `run`

## See also

- [[_index]]
- [[cw-server]]
