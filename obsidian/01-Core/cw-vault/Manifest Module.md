---
tags: [core, cw-vault, module]
---

# Manifest Module

> Machine metadata stored in `obsidian/.codewiki/`.

## Source

- `crates/cw-vault/src/manifest.rs` — module `cw_vault::manifest`

## How it works

Main items:

- [[VaultMeta]] — everything under `.codewiki/`, loaded.
- `NoteEntry` — struct
- `Manifest` — struct
- `Origin` — enum
- `SourceRef` — A source file a note describes, with the content hash seen at generation.
- `MANIFEST_SCHEMA` — const
- `body_hash` — hash used to tell whether a note changed on disk.

## Depends on

- [[Hash Module]] — uses `normalize_text`, `sha256_hex`
- [[Error Module]] — uses `Result`
- [[NoteKind]] — uses `NoteKind`

## Used by

- [[VaultMeta]] — from `VaultMeta`, `by_path`, `load`, `notes_for_source`
- [[Generate Module]] — from `adopt_existing`, `cited_sources`, `generate`
- [[Query Module]] — from `NoteInfo`, `info`, `title_of`
- [[Lifecycle Module]] — from `action`, `state`
- [[Proposal Module]] — from `approve`, `create`

## See also

- [[_index]]
- [[cw-vault]]
