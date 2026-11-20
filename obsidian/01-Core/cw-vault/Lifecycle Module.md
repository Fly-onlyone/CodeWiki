---
tags: [core, cw-vault, module]
---

# Lifecycle Module

> Note lifecycle: what may be done to a note given its state on disk.

## Source

- `crates/cw-vault/src/lifecycle.rs` — module `cw_vault::lifecycle`

## How it works

1. `generate_with` in [[Generate Module]] and `doc_drift` in [[Health Module]], and 1 more, call `state`. `state` calls `body_hash` in [[Manifest Module]].
2. `generate_with` in [[Generate Module]] calls `action`.

## Items

- `pub enum NoteState` — public enum; used by `NoteInfo`, `action`, `state`.
- `pub enum Action` — what the updater should do with a note.
- `pub fn state(entry: &NoteEntry, disk: Option<&str>, sources_changed: bool) -> NoteState` — classify a note. `disk` is the current file text, if the file exists. `sources_changed` is true when any cited source hash differs from the manifest.
- `pub fn action(entry: &NoteEntry, state: NoteState, new_input_hash: &str) -> Action` — decide the action. `new_input_hash` is the hash of the inputs a regeneration would use now.

## Depends on

- [[Manifest Module]] — uses `NoteEntry`, `body_hash`

## Used by

- [[Query Module]] — from `NoteInfo`, `info`
- [[Generate Module]] — from `generate_with`
- [[Health Module]] — from `doc_drift`

## See also

- [[_index]]
- [[cw-vault]]
