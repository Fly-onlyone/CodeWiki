---
tags: [integration]
---

# windows_registry

> External crate `windows_registry`, imported by 3 source files.

## What it does

`windows_registry` reads and writes the Windows registry. CodeWiki uses it in three places, always under the current user's hive, so no administrator rights are needed. One sets the app to start when you sign in. One gives Windows the name and icon for the app's notifications. One keeps the version shown in Settings > Apps up to date after an update.

## Used for

Imported by 3 source files. Main users:

- [[Apps Entry Module]] — imports `CURRENT_USER`
- [[Autostart Module]] — imports `CURRENT_USER`
- [[Identity Module]] — imports `CURRENT_USER`, `Key`, `Type`

## Configuration

- `crates/codewiki-app/Cargo.toml`, `crates/cw-update/Cargo.toml` — declare the dependency: version `0.100`, locked `0.100.0`

## Wire-up

The [[Autostart Module]] writes one value named `CodeWiki` under the user's `Run` key. Its text is the quoted program path plus `--tray`. That value is the whole setting. `enabled` reads it and `matches` compares it with this installation's command, ignoring case.

The [[Identity Module]] writes `DisplayName` and `IconUri` under the key for the app's notification id. `stale` finds which values are missing or differ, and `update` writes only those. The name is a plain string and the icon path is an expandable string, as Microsoft's example does.

The [[Apps Entry Module]] opens the setup program's uninstall key and sets `DisplayVersion`.

## Auth mode

No sign-in. Access is the signed-in Windows user's own registry hive.

## Gotchas

- `record_version` opens the key and never creates it. An install made by `install.cmd` has no entry and must not get one, so the call returns false.
- Autostart works only for an installed copy. A build in the target folder would leave a value pointing at a path that may vanish.
- The app does not read the Startup apps switch in Windows Settings. If the user turns it off there, the app still reports the setting as on.
- Tests use their own keys under a test path and remove them, never the real Run key.
- Outside Windows, `record_version` does nothing and returns false.

## See also

- [[_index]]
