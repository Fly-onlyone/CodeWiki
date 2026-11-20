---
tags: [core, cw-server, module]
---

# Api Module

> Routes and handlers.

## Source

- `crates/cw-server/src/api.rs` — module `cw_server::api`

## How it works

Main items:

- `routes` — function
- `ApiError` — struct
- `blocking` — run blocking work (SQLite, file I/O, parsing) off the async runtime.
- `Repo` — resolved repository: its root folder and its index.
- `Api` — type alias
- `default_search_limit` — function
- `start_model_job` — start a job and answer 202 with it; its progress is in `GET /v1/models`.
- `models_closed` — the answer of every model route on a backend that listens beyond this machine.

## Depends on

- [[Query Module]] — uses `Answer`, `Context`, `NoteContent`, `NoteInfo`
- [[Gitintel Module]] — uses `BlastRadius`, `ChangeRisk`, `FileRisk`, `GitIntel`
- [[Proposal Module]] — uses `Proposal`, `approve`, `get`, `list`
- [[Index]] — uses `Index`, `edges`, `files`, `find_symbols`
- [[Coverage Module]] — uses `CoverageReport`, `import`, `import_text`, `load`
- [[Health Module]] — uses `DeadSymbol`, `DocReport`, `FileHealth`, `dead_code`
- [[Models Module]] — uses `Kind`, `after_update`, `describe`, `remove`
- [[Repos Module]] — uses `RepoEntry`, `add`, `get`, `load`

## Used by

- [[cw-server]] — from `router`, `serve`

## See also

- [[_index]]
- [[cw-server]]
