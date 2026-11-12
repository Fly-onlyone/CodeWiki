---
tags: [core, cw-vault, module]
---

# Registry Module

> The note-name registry.

## Source

- `crates/cw-vault/src/registry.rs` — module `cw_vault::registry`

## How it works

Main items:

- [[Registry]] — maps note names (file stem, compared case-insensitively because Windows file names and Obsidian links are) to entries.
- [[RegistryEntry]] — struct
- `sanitize_name` — make a title safe to use as a file name and link target.
- `FORBIDDEN` — characters Obsidian or Windows forbid in file names or link targets.
- `folder` — function
- `join` — function
- `stem` — function

## Used by

- [[Registry]] — from `insert`, `reserve`, `resolve`
- [[Generate Module]] — from `generate`, `planned_path`
- [[Decisions Module]] — from `create`

## See also

- [[_index]]
- [[cw-vault]]
