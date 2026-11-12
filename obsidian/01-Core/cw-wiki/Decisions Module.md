---
tags: [core, cw-wiki, module]
---

# Decisions Module

> Decision records and the "why" query.

## Source

- `crates/cw-wiki/src/decisions.rs` — module `cw_wiki::decisions`

## How it works

Main items:

- `CommitInfo` — struct
- `list` — every decision record in the repository and its vault, by path.
- `Decision` — struct
- `create` — create a new decision note from the standard template and return its vault-relative path.
- `Rationale` — struct
- `why` — why is this code the way it is?
- `DECISIONS_ZONE` — vault folder new decisions are written to.
- `Why` — struct

## Depends on

- [[Error]] — uses `invalid`, `io`, `not_found`
- [[Index]] — uses `Index`, `files`, `find_symbols`
- [[Gitintel Module]] — uses `GitIntel`, `load`
- [[VaultMeta]] — uses `load`, `notes_for_source`
- [[Note]] — uses `Note`, `parse`
- [[cw-core]] — uses `VAULT_DIR`
- [[Error Module]] — uses `Result`
- [[Registry Module]] — uses `sanitize_name`

## Used by

- [[cw-wiki]] — from `a_decision_goes_stale_when_its_code_changes_later`, `lists_decisions_from_adr_files_and_vault_notes`, `why_gathers_decisions_rationale_and_commits`
- [[Api Module]] — from `create_decision`, `list_decisions`, `why`
- [[codewiki]] — from `run`
- [[Ci Module]] — from `check`

## See also

- [[_index]]
- [[cw-wiki]]
