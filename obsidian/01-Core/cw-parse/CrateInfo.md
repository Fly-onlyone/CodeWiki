---
tags: [core, cw-parse]
---

# CrateInfo

> One Cargo package found in the repository.

## What it does

`CrateInfo` describes one Cargo package found in a repository: its name as used in Rust paths (dashes become underscores), its name as written in `Cargo.toml`, its folder, its description, and whether it has a library and a binary. It also lists dependencies. `deps` holds only `[dependencies]`. `code_deps` holds every crate the package's code can name, which adds dev and build dependencies and the ones in `[target.*]` tables. The resolver, the scan, the test map and the overview use it to map a file to its crate and a path like `cw_core::model` to a crate.

## Source

- `crates/cw-parse/src/workspace.rs` — definition of `CrateInfo`

## How it works

`parse_manifest` in [[Workspace Module]] fills one `CrateInfo` from a `Cargo.toml`. `dependency_names` reads the three dependency tables (normal, dev, build), at the top level and in target tables, and turns each name into its path form. `owning_crate` and `module_path` use `dir` to find which crate a file belongs to. An empty `dir` means the crate sits at the repository root.

## Items

Declared as `pub struct CrateInfo`.

## Used by

- [[Workspace Module]] — from `module_path`, `owning_crate`, `parse_manifest`
- [[Testmap Module]] — from `Env`, `command`
- [[Scan Module]] — from `RepoFacts`
- [[Query Module]] — from `Overview`
- [[Index]] — from `crates`
- [[Resolver]] — from `with_crates`

## Gotchas

- `deps` and `code_deps` differ on purpose. Using `deps` for "who can this code call" misses test and build code. Use `code_deps` for that.
- `name` is the path form and `package` is the manifest form. Mixing them breaks lookups for crates with a dash, such as `cw-core` and `cw_core`.
- `code_deps` has a serde default, so older saved data without it loads as an empty list.

## See also

- [[_index]]
- [[Workspace Module]]
