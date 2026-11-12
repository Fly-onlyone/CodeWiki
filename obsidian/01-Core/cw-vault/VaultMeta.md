---
tags: [core, cw-vault]
---

# VaultMeta

> Everything under `.codewiki/`, loaded.

## Source

- `crates/cw-vault/src/manifest.rs` — definition of `VaultMeta`

## How it works

Declared as `pub struct VaultMeta`.

Public API:

- `pub fn load(vault: &Path) -> Result<Self>` — load metadata; a vault with none yields the default (empty) state.
- `pub fn registry(&self) -> Result<Registry>` — build the name registry from the note entries.
- `pub fn notes_for_source(&self, source_path: &str) -> Vec<&NoteEntry>` — ids of notes that cite `source_path`.
- `pub fn save(&self, vault: &Path) -> Result<()>` — write metadata.
- `pub fn dir(vault: &Path) -> PathBuf`
- `pub fn by_path(&self, path: &str) -> Option<&NoteEntry>`

## Depends on

- [[Manifest Module]] — uses `Json`, `Manifest`, `NoteEntry`, `default`
- [[Registry]] — uses `Registry`, `entries`, `new`
- [[Error]] — uses `external`, `io`
- [[cw-core]] — uses `MANIFEST_DIR`
- [[Error Module]] — uses `Result`
- [[RegistryEntry]] — uses `RegistryEntry`

## Used by

- [[cw-wiki]] — from `a_type_that_moves_between_crates_keeps_the_vault_consistent`, `assert_notes`, `crate_and_flow_notes_get_valid_illustrations`, `existing_vault_is_adopted_without_changes`
- [[Query Module]] — from `list_notes`, `notes_citing`, `read_note`, `status`
- [[Api Module]] — from `approve_proposal`, `reject_proposal`
- [[Decisions Module]] — from `list`, `why`
- [[Generate Module]] — from `adopt_existing`, `generate`
- [[Service Module]] — from `index_notes`, `is_stale`
- [[codewiki]] — from `run`
- [[Proposal Module]] — from `proposals_dir`

## See also

- [[_index]]
- [[Manifest Module]]
