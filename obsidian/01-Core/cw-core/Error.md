---
tags: [core, cw-core]
---

# Error

> `Error` enum defined in `cw_core::error` with 4 methods.

## Source

- `crates/cw-core/src/error.rs` — definition of `Error`

## How it works

Declared as `pub enum Error`.

Public API:

- `pub fn external(context: impl Into<String>, err: impl std::fmt::Display) -> Self` — wrap an error from a third-party library with the operation that failed.
- `pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self`
- `pub fn invalid(message: impl Into<String>) -> Self`
- `pub fn not_found(what: impl Into<String>) -> Self`

## Used by

- [[Proposal Module]] — from `approve`, `check_id`, `create`, `get`
- [[Generate Module]] — from `adopt_existing`, `generate`, `read_optional`, `walk`
- [[Api Module]] — from `ApiError`, `blocking`, `git_hook`, `import_coverage`
- [[Repos Module]] — from `add`, `get`, `load`, `remove`
- [[Query Module]] — from `check_vault_path`, `dependency_path`, `read_note`, `symbol_detail`
- [[Coverage Module]] — from `import`, `parse_cobertura`, `parse_lcov`, `save`
- [[cw-embed]] — from `checked`, `from_config`, `post_json`
- [[Serve Module]] — from `publish`, `random_token`, `run`

## See also

- [[_index]]
- [[Error Module]]
