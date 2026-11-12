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
- [[Proposal Module]] — from `approve`, `check_id`, `create`, `get`
- [[Coverage Module]] — from `import`, `import_text`, `load`, `parse`
- [[Service Module]] — from `built`, `index_notes`, `is_stale`, `open_index`
- [[Generate Module]] — from `adopt_existing`, `generate`, `list_notes`, `read_optional`
- [[Gitintel Module]] — from `blast_radius`, `change_risk`, `dependents_by_file`, `file_risk`
- [[cw-index]] — from `document_opening`, `nearest`, `replace_documents`, `search`

## See also

- [[_index]]
- [[cw-core]]
