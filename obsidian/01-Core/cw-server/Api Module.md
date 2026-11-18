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
- `Repo` — resolved repository: its root folder and its index.
- `blocking` — run blocking work (SQLite, file I/O, parsing) off the async runtime.
- `Api` — type alias
- `default_search_limit` — function
- `SLOW_REQUEST` — A request that takes longer than this is logged at Info.
- `GraphNode` — struct

## Depends on

- [[Query Module]] — uses `Answer`, `Context`, `NoteContent`, `NoteInfo`
- [[Gitintel Module]] — uses `BlastRadius`, `ChangeRisk`, `FileRisk`, `GitIntel`
- [[Proposal Module]] — uses `Proposal`, `approve`, `get`, `list`
- [[Index]] — uses `Index`, `edges`, `files`, `find_symbols`
- [[Coverage Module]] — uses `CoverageReport`, `import`, `import_text`, `load`
- [[Health Module]] — uses `DeadSymbol`, `DocReport`, `FileHealth`, `dead_code`
- [[Repos Module]] — uses `RepoEntry`, `add`, `get`, `load`
- [[Settings Module (cw-server)]] — uses `change`, `describe`, `resolved`, `test_embedder`

## Used by

- [[cw-server]] — from `router`, `serve`

## See also

- [[_index]]
- [[cw-server]]
