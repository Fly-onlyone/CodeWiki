---
tags: [core, cw-server, module]
---

# Workspace Module (cw-server)

> Views across every repository the backend manages.

## Source

- `crates/cw-server/src/workspace.rs` — module `cw_server::workspace`

## How it works

Main items:

- `WorkspaceRepo` — one repository in the workspace view.
- `SharedDependency` — an external package more than one repository uses.
- `RepoError` — A repository left out of a workspace result, and why.
- `repo_contracts` — HTTP contracts of one registered repository.
- `workspace` — one entry per registered repository, and the external packages two or more of them use.
- `workspace_contracts` — contracts of every registered repository, matched against each other.
- `Workspace` — struct
- `WorkspaceContractsView` — contracts matched across the workspace.

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
