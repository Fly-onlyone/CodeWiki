---
tags: [core, cw-parse, module]
---

# Scan Module

> Walk a repository and produce every fact the index needs.

## Source

- `crates/cw-parse/src/scan.rs` — module `cw_parse::scan`

## How it works

Main items:

- `scan_repo` — scan the whole repository at `root`.
- `RepoFacts` — struct
- `list_files` — list indexable files under `root`, sorted, honoring `.gitignore`.
- `relative` — Repo-relative path with forward slashes.
- `inner_doc` — leading `//!` documentation of a file.
- `SKIP_DIRS` — directories never worth indexing, on top of what `.gitignore` excludes.
- `MAX_SOURCE_BYTES` — files larger than this are recorded but not read as source.

## Depends on

- [[Lang Module]] — uses `link_imports`, `python_module`, `resolve_ts_import`, `ts_module`
- [[Workspace Module]] — uses `CrateInfo`, `module_path`, `parse_manifest`
- [[Model Module]] — uses `Edge`, `FileRecord`
- [[Error]] — uses `external`, `io`
- [[Resolver]] — uses `new`, `resolve`
- [[cw-core]] — uses `VAULT_DIR`
- [[Error Module]] — uses `Result`
- [[Hash Module]] — uses `sha256_hex`

## Used by

- [[Repo]] — from `Repo`, `at`, `build`, `new`
- [[cw-wiki]] — from `reports_documentation_drift_and_coverage`, `run`
- [[Analyze Module]] — from `analyze`, `dependency_graph`
- [[Generate Module]] — from `adopt_existing`, `generate`
- [[Service Module]] — from `reindex`
- [[Units Module]] — from `discover`
- [[Index]] — from `replace_facts`

## See also

- [[_index]]
- [[cw-parse]]
