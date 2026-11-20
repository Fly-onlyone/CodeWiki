---
tags: [core, cw-parse, module]
---

# Workspace Module

> Cargo package discovery and the file-to-module mapping.

## Source

- `crates/cw-parse/src/workspace.rs` — module `cw_parse::workspace`

## How it works

1. `scan_repo` in [[Scan Module]] and `command` in [[Testmap Module]] call `module_path`. `module_path` calls `owning_crate`, then `segments_of`.
2. `scan_repo` in [[Scan Module]] calls `parse_manifest`. `parse_manifest` calls `Error::external` in [[Error]], then `dependency_names`.
3. `cargo` in [[Manifests Module]] calls `dependency_specs`. `dependency_specs` calls `dep_spec`.
4. `Packages::read` in [[Manifests Module]] calls `locked_versions`.
5. `Machine::describe` in [[Machine]] calls `package_description`.

1 more entry point: `owning_crate`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub struct CrateInfo` — one Cargo package found in the repository.
- `const DEPENDENCY_TABLES: [&str; 3]` — the tables of a manifest, or of one of its `[target.*]` tables, that name dependencies.
- `fn dependency_names(table: Option<&toml::Value>) -> Vec<String>` — the dependency names in one table, as used in paths.
- `pub fn parse_manifest(dir: &str, text: &str, files: &[String]) -> Result<Option<CrateInfo>>` — parse a `Cargo.toml`. Returns `None` for a virtual workspace manifest.
- `pub struct DepSpec` — what a manifest asks of one dependency.
- `fn dep_spec(key: &str, value: &toml::Value, workspace: Option<&toml::Table>) -> DepSpec` — one entry of a dependency table, with what `[workspace.dependencies]` says when the entry is `workspace = true`.
- `pub fn dependency_specs(manifest: &str, workspace_manifest: Option<&str>) -> Vec<DepSpec>` — every dependency a `Cargo.toml` names, in name order: its normal, dev and build dependencies, those of `[target.*]` tables too. A dependency named in several tables is one entry. `workspace_manifest` is the text of the workspace's root manifest, which `workspace = true` refers to. A manifest that does not parse names none.
- `pub struct Locked` — the versions a `Cargo.lock` holds of one package.
- `pub fn locked_versions(lock: &str) -> BTreeMap<String, Locked>` — the packages of a `Cargo.lock`, by name as used in paths. A lock file that does not parse holds none.
- `pub fn package_description(manifest: &str) -> Option<String>` — the `description` of the package a `Cargo.toml` defines, on one line.
- `pub fn owning_crate<'a>(crates: &'a [CrateInfo], file: &str) -> Option<&'a CrateInfo>` — the crate that owns `file`: the one with the longest matching directory.
- `pub fn module_path(crates: &[CrateInfo], file: &str) -> String` — qualified module path defined by a Rust source file.
- `fn segments_of(path: &str) -> Vec<String>` — private function, returns `Vec<String>`; called by `module_path`.

## Depends on

- [[Error Module]] — uses `Result`
- [[Error]] — uses `external`

## Used by

- [[Manifests Module]] — from `Packages`, `cargo`, `read`
- [[Scan Module]] — from `RepoFacts`, `scan_repo`
- [[Query Module]] — from `Overview`
- [[Testmap Module]] — from `command`
- [[Index]] — from `crates`
- [[Resolver]] — from `with_crates`
- [[Machine]] — from `describe`

## See also

- [[_index]]
- [[cw-parse]]
