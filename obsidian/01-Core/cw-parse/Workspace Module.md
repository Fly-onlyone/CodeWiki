---
tags: [core, cw-parse, module]
---

# Workspace Module

> Cargo package discovery and the file-to-module mapping.

## Source

- `crates/cw-parse/src/workspace.rs` — module `cw_parse::workspace`

## How it works

Main items:

- [[CrateInfo]] — one Cargo package found in the repository.
- `owning_crate` — the crate that owns `file`: the one with the longest matching directory.
- `module_path` — qualified module path defined by a Rust source file.
- `parse_manifest` — parse a `Cargo.toml`.
- `segments_of` — function
- `dependency_names` — the dependency names in one table, as used in paths.
- `DEPENDENCY_TABLES` — the tables of a manifest, or of one of its `[target.*]` tables, that name dependencies.

## Depends on

- [[Error Module]] — uses `Result`
- [[Error]] — uses `external`
- [[CrateInfo]] — uses `CrateInfo`

## Used by

- [[Scan Module]] — from `scan_repo`
- [[Testmap Module]] — from `command`

## See also

- [[_index]]
- [[cw-parse]]
