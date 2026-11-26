---
tags: [core, cw-parse, module]
---

# Scan Module

> Walk a repository and produce every fact the index needs.

## What it does

This module walks a whole repository and produces every structural fact the index stores: files, crates, symbols, edges between them, and the outside packages each file uses. `scan_repo` is the entry point. Anything that needs to know what is in the code, such as a reindex or a wiki sync, starts here. The result is a `RepoFacts` value that the [[Index]] writes to the database.

## Source

- `crates/cw-parse/src/scan.rs` — module `cw_parse::scan`

## How it works

`list_files` walks the folder with the `ignore` crate. It honors `.gitignore` and a `.codewikiignore` file, skips `.git`, `target`, `node_modules` and a few more, and skips the top-level vault folder. The result is sorted.

`scan_repo` first reads every `Cargo.toml` through `parse_manifest` in [[Workspace Module]]. A broken manifest is skipped, not fatal. Then for each file it records a path, hash, byte count and line count. It parses the file by language: Rust through `parse_rust`, and Python, TypeScript or JavaScript and Java through their own parsers. Each parser's symbols are added to the list.

When all files are parsed, `link_imports` ([[Lang Module]]) sorts out Python and Java imports. Then a [[Resolver]] is built from all symbols and files, and `resolve` returns the edges and outside uses. Symbols are sorted by qualified name at the end.

## Items

- `const MAX_SOURCE_BYTES: u64` — files larger than this are recorded but not read as source.
- `const SKIP_DIRS: [&str; 6]` — directories never worth indexing, on top of what `.gitignore` excludes.
- `pub struct RepoFacts` — public struct; used by `Index::replace_facts`, `Repo`, `Repo::at`, `Repo::build` and 9 more.
- `fn relative(root: &Path, path: &Path) -> Option<String>` — Repo-relative path with forward slashes.
- `pub fn list_files(root: &Path) -> Result<Vec<String>>` — list indexable files under `root`, sorted, honoring `.gitignore`.
- `fn inner_doc(source: &str) -> String` — leading `//!` documentation of a file.
- `pub fn scan_repo(root: &Path) -> Result<RepoFacts>` — scan the whole repository at `root`.

## Depends on

- [[Lang Module]] — uses `link_imports`, `python_module`, `resolve_ts_import`, `ts_module`
- [[Workspace Module]] — uses `CrateInfo`, `module_path`, `parse_manifest`
- [[Resolver]] — uses `new`, `resolve`, `with_crates`
- [[Model Module]] — uses `Edge`, `FileRecord`
- [[Error]] — uses `external`, `io`
- [[cw-core]] — uses `VAULT_DIR`
- [[Error Module]] — uses `Result`
- [[Hash Module]] — uses `sha256_hex`

## Used by

- [[cw-wiki]] — from `an_integration_note_says_what_is_taken_and_at_which_version`, `reports_documentation_drift_and_coverage`, `run`, `sync`
- [[Repo]] — from `Repo`, `at`, `build`, `new`
- [[Generate Module]] — from `adopt_existing`, `generate`, `generate_with`
- [[Analyze Module]] — from `analyze`, `dependency_graph`
- [[Service Module]] — from `reindex`
- [[Units Module]] — from `discover`
- [[Index]] — from `replace_facts`

## Gotchas

- A file above 1.5 MB, or one that is not valid UTF-8, is recorded but not parsed. Large files get an empty hash and zero lines.
- When two symbols share a qualified name (overloads, `#[cfg]` twins, inline and file modules), only the first is kept.
- Rust files get their module symbol from the scanner, with the file's `//!` doc as its text. The other languages' parsers add their own.
- A file with syntax errors still gives facts, and is listed in `files_with_errors`.
- A read error on one file stops the whole scan, because the error is returned, not skipped.
- JavaScript is parsed with the TSX grammar.

## See also

- [[_index]]
- [[cw-parse]]
