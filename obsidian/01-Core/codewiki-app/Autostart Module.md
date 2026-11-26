---
tags: [core, codewiki-app, module]
---

# Autostart Module

> Starting the app when the user signs in to Windows.

## What it does

Windows runs what the values under the user's `Run` key say. The app's value is its program with `--tray`, so that it starts in the tray, with no window on screen. That value is the whole setting: there is no file beside it that could disagree, and whoever removes it by hand (or with `uninstall.ps1`) has turned the setting off.

Only an installed copy offers this. A build from the sources would leave behind a value that names a program in `target\`, which the next build replaces or a cleanup deletes.

Windows has a second switch, "Startup apps" in its settings and in the Task Manager, which keeps a program from starting without touching this value. The app does not read it: off there, the setting here still says on.

## Source

- `crates/codewiki-app/src/autostart.rs` — module `codewiki_app::autostart`

## How it works

The whole setting is one registry value, `CodeWiki`, under the user's `Run` key. Its data is `command`: the quoted path of `codewiki-app.exe` in the installation folder, then `--tray`.

Reading: `enabled` asks `installed_dir` in [[cw-update]] for the installation folder. A copy that is not installed has none, so `enabled` answers `None` and the settings page does not offer the switch. Otherwise `read` fetches the value and `matches` compares it, ignoring case and outer spaces, with what `command` would write.

Writing: `set(true)` writes `command`; `set(false)` calls `remove`, which deletes the value and does nothing when it is already absent. Errors come back as text for the settings page to show.

The app view calls `enabled` to draw the switch and to match palette entries, and calls `set` when the user flips the switch.

## Items

- `const RUN_KEY: &str` — the key whose values Windows runs at sign-in, under the user's hive.
- `const VALUE: &str` — the name of the app's value.
- `const PROGRAM: &str` — the program's file name in an installation.
- `pub fn command(install: &Path) -> String` — what Windows is to run for the installation in `install`: the program, quoted because the folder may hold a space, started for the tray.
- `pub fn matches(value: Option<&str>, install: &Path) -> bool` — whether `value`, as read from the `Run` key, starts the installation in `install`. Windows reads paths without regard to case. A value that starts another copy is not this installation's setting.
- `pub fn enabled() -> Option<bool>` — whether Windows starts this installation at sign-in. `None` for a copy that is not installed, which cannot be started this way.
- `pub fn set(on: bool) -> Result<(), String>` — turn the start at sign-in on or off for this installation.
- `fn read(key: &str) -> Option<String>` — private function, returns `Option<String>`; called by `enabled`, `remove`.
- `fn write(key: &str, command: &str) -> Result<(), String>` — private function, returns `Result<(), String>`; called by `set`.
- `fn remove(key: &str) -> Result<(), String>` — private function, returns `Result<(), String>`; called by `set`.

## Depends on

- [[cw-update]] — uses `installed_dir`

## Used by

- [[codewiki-app]] — from `palette_matches`, `run_action`, `settings_frame`

## Gotchas

- Only an installed copy can use it. From a source build, `set` refuses, so no value naming a file in `target\` is left behind.
- `matches` is strict on purpose. A value that starts another copy, or the same program without `--tray`, counts as off for this installation.
- Windows' own "Startup apps" switch is not read. If the user turns it off there, the setting here still says on, though nothing starts.
- Deleting the value by hand or with `uninstall.ps1` turns the setting off. No other file holds a second copy of the state.
- The unit test for the registry works on its own key under `Software\CodeWiki-test` and removes it afterwards, never the real `Run` key.

## See also

- [[_index]]
- [[codewiki-app]]
