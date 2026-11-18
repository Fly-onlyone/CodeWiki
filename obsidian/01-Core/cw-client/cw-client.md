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

Modules:

- [[Remote Module]] — A backend on another machine, or in Docker, that the user chose in the app instead of naming it with `CODEWIKI_URL`.

Defined at the crate root:

- [[Backend]] — struct
- `Origin` — how a client came to the backend it uses.
- `encode` — Percent-encode a query-string value.
- `saved_remote` — the backend saved in the app's settings, without its token.
- `env_remote` — the address `CODEWIKI_URL` names, when it is set.
- `uses_remote` — whether clients use a backend in Docker or on another machine, named by `CODEWIKI_URL` or saved in the app's settings, rather than one on this machine.
- `check_remote` — check that a CodeWiki backend answers at `url` and accepts `token`, and return its version.
- `save_remote` — save a backend's address and token and make every client on this machine use it: the app and an agent's MCP bridge alike.

## Depends on

- [[Remote Module]] — uses `Remote`, `forget_in`, `normalized`, `save_in`
- [[Backend]] — uses `new`, `status`
- [[Paths Module]] — uses `data_dir`

## Used by

- [[codewiki-app]] — from `backend_to_stop`, `connect`, `connect_remote`, `install_update`
- [[codewiki-mcp]] — from `get_architecture`, `get_blast_radius`, `get_context`, `get_dependency_path`
- [[Backend]] — from `Backend`, `answers`, `connect`, `named`
- [[Markdown Module]] — from `encode`
- [[Settings Pages Module]] — from `Model`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
