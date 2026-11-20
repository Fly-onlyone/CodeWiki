---
tags: [core, cw-server, module]
---

# Repos Module

> The list of repositories this backend manages, stored as `repos.json`.

## Source

- `crates/cw-server/src/repos.rs` — module `cw_server::repos`

## How it works

Main items:

- `get` — function
- `RepoEntry` — struct
- `load` — function
- `remove` — forget a repository.
- `add` — add a repository by path.
- `file` — function
- `save` — function

## Depends on

- [[Error]] — uses `external`, `invalid`, `io`, `not_found`
- [[Error Module]] — uses `Result`
- [[Paths Module]] — uses `repo_id`

## Used by

- [[Api Module]] — from `ListedRepo`, `add_repo`, `get_repo`, `git_hook`
- [[Workspace Module (cw-server)]] — from `open`, `repo_contracts`, `summarize`, `workspace`
- [[Models Module]] — from `embed_all`, `embed_repo`, `resume`

## See also

- [[_index]]
- [[cw-server]]
