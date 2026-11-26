---
tags: [core, cw-core]
---

# Settings

> What `settings.json` holds.

## What it does

`Settings` is the content of `settings.json`: the settings the backend keeps for its user. It reads and writes values by [[Key]]. Parts of the file it does not know are kept as they are, so an older program does not drop what a newer one wrote. Secrets such as API keys are stored encrypted, with a readable hint of their last characters.

## Source

- `crates/cw-core/src/settings.rs` — definition of `Settings`

## How it works

`value` returns the saved text of a setting that is not a secret. A blank value counts as none. `secret` returns the saved form of a secret: the encrypted text and its hint, so a page can say which key is saved without decrypting anything.

`set` trims the value. A blank value clears the setting. A secret is encrypted with `protect` and given a `hint`, in the [[Secret Module]], then stored. Other values are stored as text. `put` writes into the section, replacing a section that is not an object. `clear` removes the field and removes the section when it is empty.

The module's `update` function loads the file, applies a change and writes the whole file back through a temporary file and a rename. If the change fails, nothing is written. It also removes the retired sections and writes the layout version.

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

## Gotchas

- `set` for a secret fails on a system that cannot protect one. The error comes from the [[Secret Module]]. Nothing is saved then.
- A file that cannot be read is moved to `settings.broken.json` before a new one is written, so its owner can get it back. `load` alone never fails: a missing or damaged file gives empty settings.
- `value` returns `None` for a secret and `secret` returns `None` for a plain setting. Use the one that matches the key.
- Retired sections from the old AI writer (`llm`, `anthropic`, `cliproxy`) are removed on every write, because a key no program uses should not lie on a disk.

## See also

- [[_index]]
- [[Settings Module (cw-core)]]
