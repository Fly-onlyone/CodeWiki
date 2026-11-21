---
tags: [core, cw-parse, module]
---

# Scan Module

> Walk a repository and produce every fact the index needs.

## Source

- `crates/cw-parse/src/scan.rs` — module `cw_parse::scan`

## How it works

1. `reindex` in [[Service Module]] calls `scan_repo`. `scan_repo` calls `list_files`, then `Error::io` in [[Error]], then `parse_manifest` in [[Workspace Module]], then `Language::from_path` in [[Language]], then `sha256_hex` in [[Hash Module]], then `python_module` in [[Lang Module]], and 10 more. `list_files` calls `Error::external` in [[Error]], then `relative`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

## See also

- [[_index]]
- [[cw-parse]]
