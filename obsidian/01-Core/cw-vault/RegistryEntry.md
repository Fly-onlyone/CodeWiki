---
tags: [core, cw-vault]
---

# RegistryEntry

> `RegistryEntry` struct defined in `cw_vault::registry`.

## Source

- `crates/cw-vault/src/registry.rs` — definition of `RegistryEntry`

## How it works

Declared as `pub struct RegistryEntry`.

## Depends on

- [[NoteKind]] — uses `NoteKind`

## Used by

- [[Registry]] — from `Registry`, `by_id`, `by_name`, `entries`
- [[VaultMeta]] — from `registry`, `save`
- [[Generate Module]] — from `generate`

## See also

- [[_index]]
- [[Registry Module]]
