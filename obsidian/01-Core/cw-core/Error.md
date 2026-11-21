---
tags: [core, cw-core]
---

# Error

> `Error` enum defined in `cw_core::error` with 4 methods.

## Source

- `crates/cw-core/src/error.rs` — definition of `Error`

## How it works

1. `run` in [[codewiki]] and `checked` in [[cw-embed]], and 46 more, call `invalid`.
2. `print_json` in [[codewiki]] and `post_json` in [[cw-embed]], and 32 more, call `external`.
3. `run` in [[codewiki]] and `publish` in [[Serve Module]], and 35 more, call `io`.
4. `run` in [[codewiki]] and `entry` in [[Models Module]], and 12 more, call `not_found`.

## Items

Declared as `pub enum Error`.

Members:

- `pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self` — public method, returns `Self`; called by `Index::open`, `Store::install`, `Store::remove`, `VaultMeta::load` and 32 more.
- `pub fn invalid(message: impl Into<String>) -> Self` — public method, returns `Self`; called by `BuiltinEmbedder::embed_query`, `BuiltinEmbedder::new`, `Embedder::embed_query`, `Jobs::begin` and 40 more.
- `pub fn not_found(what: impl Into<String>) -> Self` — public method, returns `Self`; called by `brief`, `dependency_path`, `entry`, `find_entry` and 9 more.
- `pub fn external(context: impl Into<String>, err: impl std::fmt::Display) -> Self` — wrap an error from a third-party library with the operation that failed.

## Used by

- [[Proposal Module]] — from `approve`, `check_id`, `create_as`, `get`
- [[Models Module]] — from `Failure`, `entry`, `from`, `remove`
- [[Generate Module]] — from `adopt_existing`, `generate_with`, `keep_placed_pictures`, `read_optional`
- [[Api Module]] — from `ApiError`, `blocking`, `git_hook`, `import_coverage`
- [[Query Module]] — from `check_vault_path`, `dependency_path`, `find_entry`, `read_note_as`
- [[Repos Module]] — from `add`, `get`, `load`, `remove`
- [[Settings Module (cw-server)]] — from `apply`, `change`, `checked`, `message`
- [[Explanations Module]] — from `brief`, `not_explainable`, `note_text`, `record`

## See also

- [[_index]]
- [[Error Module]]
