---
tags: [core, cw-core]
---

# Settings

> What `settings.json` holds.

## Source

- `crates/cw-core/src/settings.rs` — definition of `Settings`

## How it works

Declared as `pub struct Settings`.

Parts it does not know are kept as they are, so an older program does not drop what a newer one wrote.

Public API:

- `pub fn value(&self, key: Key) -> Option<String>` — the saved value of a setting that is not a secret.
- `pub fn clear(&mut self, key: Key)` — forget the saved value of a setting.
- `pub fn secret(&self, key: Key) -> Option<SavedSecret>` — the saved form of a secret.
- `pub fn set(&mut self, key: Key, value: &str) -> Result<()>` — save a value; a blank one clears the setting.

## Depends on

- [[Secret Module]] — uses `hint`, `protect`
- [[Settings Module (cw-core)]] — uses `SavedSecret`, `key`
- [[Error Module]] — uses `Result`
- [[Key]] — uses `Key`

## Used by

- [[Settings Module (cw-core)]] — from `load`, `read`, `update`
- [[Resolved]] — from `from_parts`, `saved_secret`
- [[Settings Module (cw-server)]] — from `apply`

## See also

- [[_index]]
- [[Settings Module (cw-core)]]
