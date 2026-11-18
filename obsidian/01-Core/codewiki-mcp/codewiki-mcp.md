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
- `pretty` — function
- `rows` — function
- `failed` — A tool call that failed: logged, and returned to the agent as an error.
- `RepoParam` — struct
- `field` — the string under `key`, or an empty string.

## Depends on

- [[Backend]] — uses `Backend`, `answers`, `connect`, `repo_id`
- [[cw-client]] — uses `encode`
- [[cw-core]] — uses `MCP_CLIENT`
- [[Logs Module]] — uses `init`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
