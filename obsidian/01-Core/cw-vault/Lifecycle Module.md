---
tags: [core, cw-vault, module]
---

# Lifecycle Module

> Note lifecycle: what may be done to a note given its state on disk.

## Source

- `crates/cw-vault/src/lifecycle.rs` — module `cw_vault::lifecycle`

## How it works

Main items:

- `NoteState` — enum
- `state` — classify a note.
- `Action` — what the updater should do with a note.
- `action` — decide the action.

## Depends on

- [[Manifest Module]] — uses `NoteEntry`, `body_hash`

## Used by

- [[Query Module]] — from `NoteInfo`, `info`
- [[Generate Module]] — from `generate`
- [[Health Module]] — from `doc_drift`

## See also

- [[_index]]
- [[cw-vault]]
