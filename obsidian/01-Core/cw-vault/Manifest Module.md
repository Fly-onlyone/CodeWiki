---
tags: [core, cw-vault, module]
---

# Manifest Module

> Machine metadata stored in `obsidian/.codewiki/`.

## What it does

Notes keep the owner's frontmatter convention (tags only), so everything the generator needs to track a note lives here instead. The files are plain text and sorted, so they diff and merge cleanly in git.

## Source

- `crates/cw-vault/src/manifest.rs` — module `cw_vault::manifest`

## How it works

1. `state` in [[Lifecycle Module]] and `approve` in [[Proposal Module]], and 3 more, call `body_hash`. `body_hash` calls `sha256_hex`, then `normalize_text` (both in [[Hash Module]]).
2. `VaultMeta::load` in [[VaultMeta]] calls `Manifest::default`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const MANIFEST_SCHEMA: u32` — public constant; used by `Manifest::default`.
- `pub struct Manifest` — public struct; used by `VaultMeta`.
- `pub enum Origin` — public enum; used by `NoteEntry`, `NoteInfo`.
- `pub struct SourceRef` — A source file a note describes, with the content hash seen at generation.
- `pub struct NoteEntry` — public struct; used by `VaultMeta`, `VaultMeta::by_path`, `VaultMeta::load`, `VaultMeta::notes_for_source` and 6 more.
- `pub fn body_hash(text: &str) -> String` — hash used to tell whether a note changed on disk.
- [[VaultMeta]] — everything under `.codewiki/`, loaded.

## Depends on

- [[Hash Module]] — uses `normalize_text`, `sha256_hex`
- [[Error Module]] — uses `Result`
- [[NoteKind]] — uses `NoteKind`

## Used by

- [[VaultMeta]] — from `VaultMeta`, `by_path`, `load`, `notes_for_source`
- [[Generate Module]] — from `adopt_existing`, `cited_sources`, `generate_with`
- [[Query Module]] — from `NoteInfo`, `info`, `title_of`
- [[Lifecycle Module]] — from `action`, `state`
- [[Proposal Module]] — from `approve`, `create`

## See also

- [[_index]]
- [[cw-vault]]
