---
tags: [core, cw-update, module]
---

# Apps Entry Module

> The setup program's entry in Windows Settings > Apps.

## What it does

The setup program shows CodeWiki in Windows Settings under Apps, with a version number. After the app updates itself, that number would go stale. This module writes the new version into the setup program's Apps entry, so Settings shows the version that is installed.

## Source

- `crates/cw-update/src/apps_entry.rs` — module `cw_update::apps_entry`

## How it works

`install` in [[cw-update]] calls `record_version` with `APPS_ENTRY`, the uninstall key under `HKEY_CURRENT_USER`. On Windows the key is opened for read and write, and `DisplayVersion` is set. The function returns `Ok(true)` when the entry existed and was updated, `Ok(false)` when there was no entry, and an error text when the write failed. Off Windows it always returns `Ok(false)`.

## Items

- `pub const APPS_ENTRY: &str` — the setup program's uninstall key under `HKEY_CURRENT_USER`.
- `pub fn record_version(key: &str, version: &str) -> Result<bool, String>` — write `version` as `DisplayVersion` of the Apps entry `key` under `HKEY_CURRENT_USER`. The key is opened, never created: an installation made by `install.cmd` has no entry and must not gain one. Returns whether the entry existed and was updated.

## Used by

- [[cw-update]] — from `install`

## Gotchas

- The key is opened, never created. An installation made by `install.cmd` has no entry and must not gain one, so a missing key is a quiet `Ok(false)`, not an error.
- Any failure to open the key counts as missing. Only a failed write is an error.
- The key name is the setup program's own id with the `_is1` suffix. Changing it detaches the entry from the setup program.

## See also

- [[_index]]
- [[cw-update]]
