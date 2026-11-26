---
tags: [core, cw-core, module]
---

# Error Module

> One error type shared by the engine crates.

## What it does

Gives the engine crates one error type, [[Error]], and one `Result<T>` alias. Index, vault, wiki and embed code all return it, so errors pass between crates with `?` and no conversion code.

## Source

- `crates/cw-core/src/error.rs` — module `cw_core::error`

## How it works

[[Error]] has four cases: file trouble with the path, invalid data, not found, and a wrapped failure from another library. Each has a constructor (`io`, `invalid`, `not_found`, `external`), so callers write `Error::invalid("...")` instead of building the variant. `external` takes the operation's name and the library error, and keeps only the error's text. The message shown is built by `thiserror`.

## Items

- `pub type Result<T> = std::result::Result<T, Error>` — public type alias; used by `Embedder::embed`, `Embedder::embed_query`, `Embedder::probe`, `Facts::load` and 195 more.
- [[Error]] — enum

## Depends on

- [[Error]] — uses `Error`

## Used by

- [[Index]] — from `collect`, `crates`, `edges`, `edges_from`
- [[Proposal Module]] — from `approve`, `check_id`, `create`, `create_agent`
- [[Query Module]] — from `answer`, `check_vault_path`, `context`, `dependency_path`
- [[cw-index]] — from `document_opening`, `documents`, `nearest`, `prune_vectors`
- [[Explanations Module]] — from `approve`, `brief`, `every`, `fits_all`
- [[Service Module]] — from `built`, `index_notes`, `is_stale`, `open_existing_index_in`
- [[cw-embed]] — from `checked`, `finish`, `from_config`, `from_env`
- [[Generate Module]] — from `adopt_existing`, `generate`, `generate_with`, `keep_placed_pictures`

## Gotchas

- `external` stores a string, so the original error type and its source chain are lost. Match on it before wrapping if you need it.
- Only `Io` carries a path. Add the path to `Invalid` messages yourself.

## See also

- [[_index]]
- [[cw-core]]
