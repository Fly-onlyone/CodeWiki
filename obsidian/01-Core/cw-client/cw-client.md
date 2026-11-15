---
tags: [core, cw-client]
---

# cw-client

> Client for the CodeWiki backend: discovery, autostart and HTTP calls.

## Source

- `crates/cw-client/Cargo.toml` — package manifest
- `crates/cw-client/src/lib.rs` — crate root

## How it works

Builds a library.

Defined at the crate root:

- [[Backend]] — struct
- `encode` — Percent-encode a query-string value.
- `uses_remote` — whether clients use a backend named by `CODEWIKI_URL`, in Docker or on another machine, rather than one on this machine.
- `agent` — function
- `health_version` — the version a backend reports, when it answers `/v1/health`.
- `URL_ENV` — explicit backend address, for a backend running in Docker or on another machine.
- `BACKEND_FILE` — const
- `STOP_TIMEOUT` — how long an old backend may take to stop after it is asked to: it first finishes the requests it is running.

## Depends on

- [[Logs Module]] — uses `open`, `path`, `timestamp`

## Used by

- [[codewiki-mcp]] — from `get_architecture`, `get_blast_radius`, `get_context`, `get_dependency_path`
- [[codewiki-app]] — from `backend_to_stop`, `install_update`, `open_file`, `open_in_obsidian`
- [[Backend]] — from `answers`, `connect`, `new`, `stop`
- [[Markdown Module]] — from `encode`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
