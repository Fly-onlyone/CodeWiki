---
tags: [core, cw-core]
---

# cw-core

> Shared identifiers, hashing, configuration and error types for CodeWiki.

## Source

- `crates/cw-core/Cargo.toml` — package manifest
- `crates/cw-core/src/lib.rs` — crate root

## How it works

Builds a library.

## Items

Modules:

- [[Error Module]] — one error type shared by the engine crates.
- [[Hash Module]] — content hashing used for change detection.
- [[Logs Module]] — log files of the programs, under `<data dir>/logs`.
- [[Model Module]] — data model shared between the parser, index, graph and API layers.
- [[Paths Module]] — locations of per-user data.
- [[Secret Module]] — secrets kept in a settings file: API keys and the token of a remote backend.
- [[Settings Module (cw-core)]] — settings the backend keeps for its user: which embedder serves semantic search, and the keys and addresses it needs.

Defined at the crate root:

- `pub const VERSION: &str` — crate version, exposed so binaries report one consistent version string.
- `pub const VAULT_DIR: &str` — folder, relative to a repository root, that holds the generated vault.
- `pub const MANIFEST_DIR: &str` — folder inside the vault that holds machine metadata. Never rendered as notes.
- `pub const CLIENT_HEADER: &str` — request header in which a client of the backend says what it is. Only the MCP bridge sends it; the backend reports when a bridge last called.
- `pub const MCP_CLIENT: &str` — what the MCP bridge sends in `CLIENT_HEADER`.

## Used by

- [[Query Module]] — from `answer`, `list_notes`, `notes_citing`, `overview`
- [[Explanations Module]] — from `approve`, `brief`, `queue`, `reject`
- [[Api Module]] — from `health`, `require_token`, `vault`
- [[Decisions Module]] — from `create`, `list`, `why`
- [[Service Module]] — from `built`, `index_notes`, `is_stale`
- [[cw-server]] — from `backend_status_needs_the_token_and_counts_only_bridge_calls`, `call_as`
- [[Update Module]] — from `check`, `describe`
- [[Briefing Module]] — from `session_start`, `vault_root`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
