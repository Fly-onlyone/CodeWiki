---
tags: [core, codewiki-mcp]
---

# codewiki-mcp

> MCP stdio bridge between coding agents and the CodeWiki backend.

## Source

- `crates/codewiki-mcp/Cargo.toml` — package manifest
- `crates/codewiki-mcp/src/main.rs` — crate root

## How it works

Builds a binary.

Defined at the crate root:

- `Bridge` — struct
- `text` — function
- `connected` — the backend in `slot`, connecting on first use.
- `rows` — function
- `pretty` — function
- `RepoParam` — struct
- `field` — the string under `key`, or an empty string.
- `ChangeParams` — struct

## Depends on

- [[Backend]] — uses `Backend`, `answers`, `connect`, `repo_id`
- [[cw-client]] — uses `encode`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
