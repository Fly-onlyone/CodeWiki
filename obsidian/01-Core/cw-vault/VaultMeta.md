---
tags: [core, cw-vault]
---

# VaultMeta

> Everything under `.codewiki/`, loaded.

## Source

- `crates/cw-vault/src/manifest.rs` — definition of `VaultMeta`

## How it works

1. `run` in [[codewiki]] and `approve_proposal` in [[Api Module]], and 14 more, call `load`. `load` calls `dir`, then `Error::external` in [[Error]], then `Manifest::default` in [[Manifest Module]], then `Error::io` in [[Error]].
2. `proposals_dir` in [[Proposal Module]] calls `dir`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct VaultMeta`.

Members:

- `pub fn dir(vault: &Path) -> PathBuf` — public method, returns `PathBuf`; called by `VaultMeta::load`, `VaultMeta::save`, `proposals_dir`.
- `pub fn load(vault: &Path) -> Result<Self>` — load metadata; a vault with none yields the default (empty) state.
- `pub fn save(&self, vault: &Path) -> Result<()>` — write metadata. `registry.json` is derived from the note entries so the two can never disagree.
- `pub fn registry(&self) -> Result<Registry>` — build the name registry from the note entries.
- `pub fn by_path(&self, path: &str) -> Option<&NoteEntry>` — public method, returns `Option<&NoteEntry>`.
- `pub fn notes_for_source(&self, source_path: &str) -> Vec<&NoteEntry>` — ids of notes that cite `source_path`.

## Depends on

- [[Manifest Module]] — uses `Json`, `Manifest`, `NoteEntry`, `default`
- [[Registry]] — uses `Registry`, `entries`, `new`
- [[Error]] — uses `external`, `io`
- [[cw-core]] — uses `MANIFEST_DIR`
- [[Error Module]] — uses `Result`
- [[RegistryEntry]] — uses `RegistryEntry`

## Used by

- [[cw-wiki]] — from `a_note_whose_writer_call_failed_is_written_again_next_run`, `a_picture_goes_when_its_note_has_nothing_left_to_show`, `a_stop_keeps_what_was_written_and_the_next_run_writes_the_rest`, `a_stop_on_an_existing_wiki_keeps_its_commit`
- [[Query Module]] — from `list_notes`, `notes_citing`, `read_note_as`, `status`
- [[Api Module]] — from `approve_proposal`, `reject_proposal`
- [[Decisions Module]] — from `list`, `why`
- [[Generate Module]] — from `adopt_existing`, `generate_with`
- [[Service Module]] — from `index_notes`, `is_stale`
- [[codewiki]] — from `run`
- [[Proposal Module]] — from `proposals_dir`

## See also

- [[_index]]
- [[Manifest Module]]
