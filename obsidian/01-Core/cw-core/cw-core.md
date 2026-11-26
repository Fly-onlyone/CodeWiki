---
tags: [core, cw-core]
---

# cw-core

> Shared identifiers, hashing, configuration and error types for CodeWiki.

## What it does

`cw-core` is the base crate of CodeWiki. It holds what every other crate needs and nothing that depends on them: the data model (files, symbols, edges), content hashing, one error type, per-user paths, logs, and the settings file with its secrets. The parser, index, graph, backend, app and MCP bridge all depend on it, so it must stay small and free of heavy dependencies.

## Source

- `crates/cw-core/Cargo.toml` — package manifest
- `crates/cw-core/src/lib.rs` — crate root

## How it works

The crate root only declares the modules and re-exports `Error`, `Result`, the hash helpers and everything in [[Model Module]], so callers write `cw_core::Symbol`. It also defines shared constants. `VAULT_DIR` and `MANIFEST_DIR` fix where the generated notes and their machine data live. `CLAIM_MINUTES` is how long a listed note stays reserved for one coding agent. `CLIENT_HEADER` and `MCP_CLIENT` let the backend see that the MCP bridge called. `VERSION` comes from the package, so all binaries report one version.

`DEV_BUILD` follows debug assertions, and `APP_NAME` follows `DEV_BUILD`. A debug build calls itself "CodeWiki Dev".

## Items

Modules:

- [[Agents Off Module]] — the list of folders closed to coding agents.
- [[Auto Explain Module]] — the switch for auto explaining.
- [[Error Module]] — one error type shared by the engine crates.
- [[Hash Module]] — content hashing used for change detection.
- [[Logs Module]] — log files of the programs, under `<data dir>/logs`.
- [[Model Module]] — data model shared between the parser, index, graph and API layers.
- [[Paths Module]] — locations of per-user data.
- [[Secret Module]] — secrets kept in a settings file: API keys and the token of a remote backend.
- [[Settings Module (cw-core)]] — settings the backend keeps for its user: which embedder serves semantic search, and the keys and addresses it needs.

Defined at the crate root:

- `pub const VERSION: &str` — crate version, exposed so binaries report one consistent version string.
- `pub const DEV_BUILD: bool` — whether this is a development build (debug assertions on). A development build keeps its own folders (`paths::APP_FOLDER`) and its own name, so that one run from the sources never shares a backend, an index or settings with an installed CodeWiki on the same machine.
- `pub const APP_NAME: &str` — the name the app shows in its title, its tray icon and its notifications.
- `pub const VAULT_DIR: &str` — folder, relative to a repository root, that holds the generated vault.
- `pub const MANIFEST_DIR: &str` — folder inside the vault that holds machine metadata. Never rendered as notes.
- `pub const CLAIM_MINUTES: u64` — how many minutes a note listed for a coding agent stays reserved for it. Other agents are not offered the note until this time has passed.
- `pub const CLIENT_HEADER: &str` — request header in which a client of the backend says what it is. Only the MCP bridge sends it; the backend reports when a bridge last called.
- `pub const MCP_CLIENT: &str` — what the MCP bridge sends in `CLIENT_HEADER`.

## Used by

- [[codewiki-app]] — from `APP_IDENTITY`, `after_hide`, `main`, `note_file`
- [[Query Module]] — from `answer`, `list_notes`, `notes_citing`, `overview`
- [[Explanations Module]] — from `approve`, `brief`, `fits_all`, `reject`
- [[Api Module]] — from `health`, `pending_proposals`, `require_token`, `vault`
- [[Decisions Module]] — from `create`, `list`, `why`
- [[Service Module]] — from `built`, `index_notes`, `is_stale`
- [[codewiki]] — from `batch_and_claim_texts_follow_the_backend`, `run`
- [[codewiki-mcp]] — from `connected`, `waiting_text`

## Gotchas

- `DEV_BUILD` is set at compile time by debug assertions. A debug build uses its own folders and name, so it never shares a backend, index or settings with an installed CodeWiki. A release build run from the sources does share them.
- `MANIFEST_DIR` is inside the vault but is never rendered as notes; do not put readable content there.
- Changing `CLAIM_MINUTES` changes how long other agents wait for a claimed note.

## See also

- [[_index]]
- [[Crate Dependency Flow]]
