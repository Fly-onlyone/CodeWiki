---
tags: [core, cw-core, module]
---

# Error Module

> One error type shared by the engine crates.

## Source

- `crates/cw-core/src/error.rs` — module `cw_core::error`

## How it works

Main items:

- [[Error]] — enum
- `Result` — type alias

## Depends on

- [[Error]] — uses `Error`

## Used by

- [[Index]] — from `collect`, `crates`, `edges`, `edges_from`
- [[Query Module]] — from `answer`, `check_vault_path`, `context`, `dependency_path`
- [[cw-index]] — from `document_opening`, `documents`, `nearest`, `prune_vectors`
- [[Proposal Module]] — from `approve`, `check_id`, `create`, `get`
- [[Service Module]] — from `built`, `index_notes`, `is_stale`, `open_existing_index_in`
- [[Coverage Module]] — from `import`, `import_text`, `load`, `parse`
- [[Semantic Module]] — from `behind`, `built`, `embed_documents`, `embed_documents_with`
- [[cw-embed]] — from `checked`, `from_config`, `from_config_in`, `from_env`

## See also

- [[_index]]
- [[cw-core]]
