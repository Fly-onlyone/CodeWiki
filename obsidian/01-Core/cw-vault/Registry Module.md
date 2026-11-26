---
tags: [core, cw-vault, module]
---

# Registry Module

> The note-name registry.

## What it does

Keeps the list of note names for a vault. Links between notes use the title only, so every title must be unique and fixed before any note body is written. This module holds one [[Registry]] of entries, each with an id, a vault path and a kind, and the helper `sanitize_name` that turns a title into a safe file name.

## Source

- `crates/cw-vault/src/registry.rs` — module `cw_vault::registry`

## How it works

`sanitize_name` replaces characters that Obsidian or Windows forbid in names or link targets (brackets, pipe, hash, colon, slashes and a few more) with spaces. It collapses repeated spaces and trims dots at the ends. The `stem` helper takes the file name without `.md`. `folder` and `join` build and split vault paths for the lookup of `_index` maps. The [[Registry]] uses them to insert, reserve and resolve names.

## Items

- `pub struct RegistryEntry` — public struct; used by `Registry`, `Registry::by_id`, `Registry::by_name`, `Registry::entries` and 6 more.
- [[Registry]] — maps note names (file stem, compared case-insensitively because Windows file names and Obsidian links are) to entries.
- `const FORBIDDEN: [char; 12]` — characters Obsidian or Windows forbid in file names or link targets.
- `pub fn sanitize_name(title: &str) -> String` — make a title safe to use as a file name and link target.
- `fn stem(path: &str) -> &str` — private function, returns `&str`; called by `Registry::insert`.
- `fn folder(path: &str) -> &str` — private function, returns `&str`; called by `Registry::resolve`.
- `fn join(dir: &str, file: &str) -> String` — private function, returns `String`; called by `Registry::resolve`.

## Depends on

- [[NoteKind]] — uses `NoteKind`

## Used by

- [[Registry]] — from `Registry`, `by_id`, `by_name`, `entries`
- [[Generate Module]] — from `generate_with`, `planned_path`
- [[VaultMeta]] — from `registry`, `save`
- [[Decisions Module]] — from `create`

## Gotchas

- Names are compared in lower case, because Windows file names and Obsidian links ignore case.
- Sanitizing can make two different titles the same name. The registry's `reserve` solves that by adding the folder name or a counter.
- Map notes are not keyed by name. See [[Registry]].

## See also

- [[_index]]
- [[cw-vault]]
