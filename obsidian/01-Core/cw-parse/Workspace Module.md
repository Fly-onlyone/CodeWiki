---
tags: [core, cw-parse, module]
---

# Workspace Module

> Cargo package discovery and the file-to-module mapping.

## What it does

Reads Cargo files and answers two questions. Which packages does the repository hold, and which Rust module does a source file define? Parsing, the indexer and the resolver use these answers to give every symbol a qualified path such as `cw_core::hash`. The module also reads dependency requirements from `Cargo.toml`, exact versions from `Cargo.lock`, and the one-line package description.

## Source

- `crates/cw-parse/src/workspace.rs` — module `cw_parse::workspace`

## How it works

`parse_manifest` parses one `Cargo.toml` and builds a `CrateInfo`. A manifest with no `[package]` table (a virtual workspace root) gives `None`. Names swap `-` for `_`, because paths use underscores. `deps` holds only `[dependencies]`. `code_deps` also holds dev and build dependencies, and those under `[target.*]` tables, sorted and without repeats.

`owning_crate` picks the crate whose directory is the longest prefix of the file. The root crate has an empty directory and matches every file. `module_path` then maps the path inside the crate:

- `src/lib.rs` gives the crate name alone.
- `src/main.rs` gives the crate name alone, or `crate::main` when the crate also has a library.
- `a/mod.rs` drops `mod`; `bin/x/main.rs` drops `main`.
- Any other file gives its path split on `/`, with `src/` removed.

With no owning crate, the file path itself becomes the module path.

`dependency_specs` returns one `DepSpec` per dependency. For `workspace = true`, `dep_spec` first reads the entry in `[workspace.dependencies]`, then the member's own entry on top. Features are added together. If a dependency appears in several tables, the entries merge: the first non-empty version wins, and `default_features` is false if any entry turns it off.

`locked_versions` groups `Cargo.lock` packages by name and keeps every version in file order.

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

## Gotchas

- `dependency_specs` and `locked_versions` hide parse errors. A bad file gives an empty result. `parse_manifest` is the only function that returns an error.
- Without the workspace root text, a `workspace = true` dependency has an empty version.
- A workspace root that is also a package looks up shared entries in itself.
- `deps` and `code_deps` differ on purpose: `deps` is for the dependency graph, `code_deps` for names the code can use.
- A binary at `src/main.rs` in a crate with a library gets the extra `main` segment. Without a library it does not.
- `module_path` slices the file path by the crate directory length. It expects repo-relative paths with forward slashes.

## See also

- [[_index]]
- [[cw-parse]]
