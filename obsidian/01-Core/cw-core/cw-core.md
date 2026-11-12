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

Modules:

- [[Error Module]] — one error type shared by the engine crates.
- [[Hash Module]] — content hashing used for change detection.
- [[Model Module]] — data model shared between the parser, index, graph and API layers.
- [[Paths Module]] — locations of per-user data.

Defined at the crate root:

- `MANIFEST_DIR` — folder inside the vault that holds machine metadata.
- `VAULT_DIR` — folder, relative to a repository root, that holds the generated vault.
- `VERSION` — crate version, exposed so binaries report one consistent version string.

## Used by

- [[Query Module]] — from `answer`, `list_notes`, `notes_citing`, `overview`
- [[Decisions Module]] — from `create`, `list`, `why`
- [[Service Module]] — from `built`, `index_notes`, `is_stale`
- [[Api Module]] — from `health`, `vault`
- [[Briefing Module]] — from `session_start`, `vault_root`
- [[Ci Module]] — from `check`, `to_sarif`
- [[codewiki]] — from `run`
- [[Answer Module]] — from `context`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
