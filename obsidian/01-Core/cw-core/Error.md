---
tags: [core, cw-core]
---

# Error

> `Error` enum defined in `cw_core::error` with 4 methods.

## Source

- `crates/cw-core/src/error.rs` — definition of `Error`

## How it works

1. `run` in [[codewiki]] and `checked` in [[cw-embed]], and 48 more, call `invalid`.
2. `print_json` in [[codewiki]] and `post_json` in [[cw-embed]], and 36 more, call `external`.
3. `run` in [[codewiki]] and `publish` in [[Serve Module]], and 30 more, call `io`.
4. `run` in [[codewiki]] and `entry` in [[Models Module]], and 10 more, call `not_found`.

## Items

Declared as `pub enum Error`.

Members:

- `pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self` — public method, returns `Self`; called by `Index::open`, `Store::install`, `Store::remove`, `VaultMeta::load` and 26 more.
- `pub fn invalid(message: impl Into<String>) -> Self` — public method, returns `Self`; called by `BuiltinEmbedder::embed_query`, `BuiltinEmbedder::new`, `Embedder::embed_query`, `Jobs::begin` and 42 more.
- `pub fn not_found(what: impl Into<String>) -> Self` — public method, returns `Self`; called by `dependency_path`, `entry`, `get`, `import` and 7 more.
- `pub fn external(context: impl Into<String>, err: impl std::fmt::Display) -> Self` — wrap an error from a third-party library with the operation that failed.

## Used by

- [[Proposal Module]] — from `approve`, `check_id`, `create`, `get`
- [[Models Module]] — from `Failure`, `entry`, `from`, `remove`
- [[Api Module]] — from `ApiError`, `blocking`, `git_hook`, `import_coverage`
- [[Settings Module (cw-server)]] — from `apply`, `change`, `checked`, `message`
- [[Generate Module]] — from `adopt_existing`, `generate_with`, `read_optional`, `walk`
- [[Repos Module]] — from `add`, `get`, `load`, `remove`
- [[Query Module]] — from `check_vault_path`, `dependency_path`, `read_note_as`, `symbol_detail`
- [[Provider Module]] — from `exchange`, `from_config`, `http_error`, `missing`

## See also

- [[_index]]
- [[Error Module]]
