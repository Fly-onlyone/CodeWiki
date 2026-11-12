---
tags: [core, cw-parse, module]
---

# Workspace Module

> Cargo package discovery and the file-to-module mapping.

## Source

- `crates/cw-parse/src/workspace.rs` — module `cw_parse::workspace`

## How it works

Main items:

- `CrateInfo` — one Cargo package found in the repository.
- `owning_crate` — the crate that owns `file`: the one with the longest matching directory.
- `module_path` — qualified module path defined by a Rust source file.
- `parse_manifest` — parse a `Cargo.toml`.
- `segments_of` — function

## Depends on

- [[Error Module]] — uses `Result`
- [[Error]] — uses `external`

## Used by

- [[Scan Module]] — from `RepoFacts`, `scan_repo`
- [[Query Module]] — from `Overview`
- [[Testmap Module]] — from `command`
- [[Index]] — from `crates`

## See also

- [[_index]]
- [[cw-parse]]
