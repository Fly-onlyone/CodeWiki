---
tags: [core, cw-core, module]
---

# Error Module

> One error type shared by the engine crates.

## Source

- `crates/cw-core/src/error.rs` — module `cw_core::error`

## Items

- `pub type Result<T> = std::result::Result<T, Error>` — public type alias; used by `BuiltinEmbedder::embed`, `BuiltinEmbedder::embed_query`, `BuiltinEmbedder::new`, `Embedder::embed` and 193 more.
- [[Error]] — enum

## Depends on

- [[Error]] — uses `Error`

## Used by

- [[Index]] — from `collect`, `crates`, `edges`, `edges_from`
- [[Proposal Module]] — from `approve`, `check_id`, `create`, `create_agent`
- [[Query Module]] — from `answer`, `check_vault_path`, `context`, `dependency_path`
- [[cw-index]] — from `document_opening`, `documents`, `nearest`, `prune_vectors`
- [[Service Module]] — from `built`, `index_notes`, `is_stale`, `open_existing_index_in`
- [[Generate Module]] — from `adopt_existing`, `generate`, `generate_with`, `keep_placed_pictures`
- [[Coverage Module]] — from `import`, `import_text`, `load`, `parse`
- [[Explanations Module]] — from `approve`, `brief`, `note_text`, `queue`

## See also

- [[_index]]
- [[cw-core]]
