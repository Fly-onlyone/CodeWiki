---
tags: [core, cw-server, module]
---

# Workspace Module (cw-server)

> Views across every repository the backend manages.

## What it does

One backend can manage many repositories. This module builds views that span all of them. The workspace view gives each repository its counts, its top hotspots and the external packages that two or more repositories share. The contracts view matches HTTP calls in one repository against routes in another. The app and the CLI use these views to show how projects relate.

## Source

- `crates/cw-server/src/workspace.rs` — module `cw_server::workspace`

## How it works

`workspace` loads the registered repositories and calls `summarize` for each. `summarize` opens the index, reads the status, takes the top three hotspots and collects external packages: declared dependencies plus crates the source imports, minus the repository's own crates. The result feeds a map from package name to the repositories that use it. Only packages used by two or more repositories become `SharedDependency` entries.

`workspace_contracts` scans each repository with `repo_contracts` and passes all results to `match_workspace`, which pairs calls with routes. `repo_contracts` serves the single-repository route.

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

## Gotchas

- A repository that cannot be read does not fail the call. The workspace view lists it with zero counts and an `error`. The contracts view lists it in `errors`.
- `open` fails with not found when the folder is gone, so a moved repository shows as an error, not as empty.
- Contracts are found by reading source text. A path built at run time is not seen.

## See also

- [[_index]]
- [[cw-server]]
