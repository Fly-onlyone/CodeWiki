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
- `agent` — function
- `BACKEND_FILE` — const
- `codewiki_binary` — the `codewiki` binary: beside this executable when installed together, otherwise whatever `PATH` provides.
- `is_healthy` — function
- `read_backend_file` — function
- `STARTUP_TIMEOUT` — const

## Used by

- [[codewiki-mcp]] — from `get_architecture`, `get_blast_radius`, `get_context`, `get_dependency_path`
- [[codewiki-app]] — from `open_file`, `open_in_obsidian`, `open_note`, `open_symbol`
- [[Backend]] — from `connect`, `new`
- [[Markdown Module]] — from `encode`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
