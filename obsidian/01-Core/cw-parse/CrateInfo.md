---
tags: [core, cw-parse]
---

# CrateInfo

> One Cargo package found in the repository.

## Source

- `crates/cw-parse/src/workspace.rs` — definition of `CrateInfo`

## How it works

Declared as `pub struct CrateInfo`.

## Used by

- [[Workspace Module]] — from `module_path`, `owning_crate`, `parse_manifest`
- [[Scan Module]] — from `RepoFacts`
- [[Query Module]] — from `Overview`
- [[Testmap Module]] — from `command`
- [[Index]] — from `crates`
- [[Resolver]] — from `with_crates`

## See also

- [[_index]]
- [[Workspace Module]]
