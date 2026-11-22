---
tags: [core, cw-update, module]
---

# Apps Entry Module

> The setup program's entry in Windows Settings > Apps.

## Source

- `crates/cw-update/src/apps_entry.rs` — module `cw_update::apps_entry`

## How it works

1. `install` in [[cw-update]] calls `record_version`.

## Items

- `pub const APPS_ENTRY: &str` — the setup program's uninstall key under `HKEY_CURRENT_USER`.
- `pub fn record_version(key: &str, version: &str) -> Result<bool, String>` — write `version` as `DisplayVersion` of the Apps entry `key` under `HKEY_CURRENT_USER`. The key is opened, never created: an installation made by `install.cmd` has no entry and must not gain one. Returns whether the entry existed and was updated.

## Used by

- [[cw-update]] — from `install`

## See also

- [[_index]]
- [[cw-update]]
