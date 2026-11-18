---
tags: [core, codewiki-app, module]
---

# Autostart Module

> Starting the app when the user signs in to Windows.

## Source

- `crates/codewiki-app/src/autostart.rs` — module `codewiki_app::autostart`

## How it works

Main items:

- `command` — what Windows is to run for the installation in `install`: the program, quoted because the folder may hold a space, started for the tray.
- `matches` — whether `value`, as read from the `Run` key, starts the installation in `install`.
- `enabled` — whether Windows starts this installation at sign-in.
- `set` — turn the start at sign-in on or off for this installation.
- `VALUE` — the name of the app's value.
- `PROGRAM` — the program's file name in an installation.
- `read` — function
- `RUN_KEY` — the key whose values Windows runs at sign-in, under the user's hive.

## Depends on

- [[cw-update]] — uses `installed_dir`

## Used by

- [[codewiki-app]] — from `palette_matches`, `run_action`, `settings_frame`

## See also

- [[_index]]
- [[codewiki-app]]
