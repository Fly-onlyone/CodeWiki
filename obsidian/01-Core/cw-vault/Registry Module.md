---
tags: [core, cw-vault, module]
---

# Registry Module

> The note-name registry.

## What it does

Wikilinks name notes by title, so titles must be unique across the vault and fixed before any note body is written. The registry is that fixed list.

## Source

- `crates/cw-vault/src/registry.rs` — module `cw_vault::registry`

## How it works

1. `create` in [[Decisions Module]] and `generate_with` in [[Generate Module]], and 2 more, call `sanitize_name`.
2. `Registry::insert` in [[Registry]] calls `stem`.
3. `Registry::resolve` in [[Registry]] calls `folder`.
4. `Registry::resolve` in [[Registry]] calls `join`.

## Items

- [[RegistryEntry]] — struct
- [[Registry]] — maps note names (file stem, compared case-insensitively because Windows file names and Obsidian links are) to entries.
- `const FORBIDDEN: [char; 12]` — characters Obsidian or Windows forbid in file names or link targets.
- `pub fn sanitize_name(title: &str) -> String` — make a title safe to use as a file name and link target.
- `fn stem(path: &str) -> &str` — private function, returns `&str`; called by `Registry::insert`.
- `fn folder(path: &str) -> &str` — private function, returns `&str`; called by `Registry::resolve`.
- `fn join(dir: &str, file: &str) -> String` — private function, returns `String`; called by `Registry::resolve`.

## Used by

- [[Registry]] — from `insert`, `reserve`, `resolve`
- [[Generate Module]] — from `generate_with`, `planned_path`
- [[Decisions Module]] — from `create`

## See also

- [[_index]]
- [[cw-vault]]
