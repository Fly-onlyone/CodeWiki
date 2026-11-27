---
tags: [core, cw-core]
---

# Error

> `Error` enum defined in `cw_core::error` with 4 methods.

## What it does

The one error enum of the engine crates, behind the `Result` alias in [[Error Module]]. Its four cases cover file trouble with a path, invalid data, a missing thing, and a failure from another library.

## Source

- `crates/cw-core/src/error.rs` — definition of `Error`

## How it works

Code builds errors with four constructors. `io` takes the path and the I/O error, so the message says where it failed. `invalid` is for bad input, bad files or refused requests. `not_found` names what is missing. `external` takes the failed operation and any displayable error, and keeps its text.

Messages are formed by `thiserror`: `I/O error at <path>: ...`, `invalid data: ...`, `not found: ...`, and `<context>: <message>`. Server code such as [[Api Module]] turns the case into an HTTP answer.

## Items

Declared as `pub enum Error`.

Members:

- `pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self` — public method, returns `Self`; called by `Index::open`, `VaultMeta::load`, `VaultMeta::save`, `adopt_existing` and 36 more.
- `pub fn invalid(message: impl Into<String>) -> Self` — public method, returns `Self`; called by `Embedder::embed_query`, `Jobs::begin`, `OllamaEmbedder::embed_query`, `OpenAiEmbedder::check_listed` and 39 more.
- `pub fn not_found(what: impl Into<String>) -> Self` — public method, returns `Self`; called by `brief`, `dependency_path`, `find_entry`, `get` and 8 more.
- `pub fn external(context: impl Into<String>, err: impl std::fmt::Display) -> Self` — wrap an error from a third-party library with the operation that failed.

## Used by

- [[Proposal Module]] — from `approve`, `check_id`, `close_retire`, `create_as`
- [[Api Module]] — from `ApiError`, `blocking`, `decide_proposals`, `git_hook`
- [[cw-embed]] — from `checked`, `finish`, `from_config`, `get_json`
- [[Generate Module]] — from `adopt_existing`, `generate_with`, `keep_placed_pictures`, `read_optional`
- [[Explanations Module]] — from `approve_keeping`, `brief`, `not_explainable`, `note_text`
- [[Query Module]] — from `check_vault_path`, `dependency_path`, `find_entry`, `read_note_as`
- [[Repos Module]] — from `add`, `get`, `load`, `remove`
- [[Settings Module (cw-server)]] — from `apply`, `change`, `checked`, `message`

## Gotchas

- `external` keeps only text, not the original error, so it cannot be inspected later.
- Messages reach users and agents as written. Write them as sentences someone can act on.
- `Invalid` carries no path. Put the file name in the message.

## See also

- [[_index]]
- [[Error Module]]
