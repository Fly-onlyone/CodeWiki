---
tags: [core, cw-vault]
---

# NoteEntry

> `NoteEntry` struct defined in `cw_vault::manifest`.

## Source

- `crates/cw-vault/src/manifest.rs` — definition of `NoteEntry`

## Items

Declared as `pub struct NoteEntry`.

## Depends on

- [[Manifest Module]] — uses `Explained`, `Origin`
- [[SourceRef]] — uses `SourceRef`
- [[NoteKind]] — uses `NoteKind`

## Used by

- [[Explanations Module]] — from `all_waiting`, `changed_since_sync`, `count`, `not_explainable`
- [[Generate Module]] — from `adopt_existing`, `generate_with`, `keep_placed_pictures`, `weave_kept`
- [[VaultMeta]] — from `VaultMeta`, `by_path`, `load`, `notes_for_source`
- [[Query Module]] — from `find_entry`, `info`, `title_of`
- [[cw-wiki]] — from `entry`, `entry_of`
- [[Lifecycle Module]] — from `action`, `state`

## See also

- [[_index]]
- [[Manifest Module]]
