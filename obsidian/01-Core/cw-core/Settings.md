---
tags: [core, cw-core]
---

# Settings

> What `settings.json` holds.

## What it does

Parts it does not know are kept as they are, so an older program does not drop what a newer one wrote.

## Source

- `crates/cw-core/src/settings.rs` — definition of `Settings`

## How it works

What the functions of this type call:

1. `value` calls `field`.
2. `secret` calls `field`.
3. `set` calls `clear`, then `protect`, then `hint` (both in [[Secret Module]]), then `put`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct Settings`.

Members:

- `fn field(&self, key: Key) -> Option<&Value>` — private method, returns `Option<&Value>`; called by `Settings::secret`, `Settings::value`.
- `pub fn value(&self, key: Key) -> Option<String>` — the saved value of a setting that is not a secret.
- `pub fn secret(&self, key: Key) -> Option<SavedSecret>` — the saved form of a secret.
- `fn put(&mut self, key: Key, value: Value)` — private method; called by `Settings::set`.
- `pub fn set(&mut self, key: Key, value: &str) -> Result<()>` — save a value; a blank one clears the setting. A secret is encrypted first, which fails on a system that cannot protect one.
- `pub fn clear(&mut self, key: Key)` — forget the saved value of a setting.

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
