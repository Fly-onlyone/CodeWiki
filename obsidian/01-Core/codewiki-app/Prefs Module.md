---
tags: [core, codewiki-app, module]
---

# Prefs Module

> Small per-user settings, one value per file under `%APPDATA%\CodeWiki` (`CodeWiki-Dev` for a development build, `cw_core::paths::APP_FOLDER`).

## What it does

This is the app's tiny settings store. Each setting is one text file named after the setting, in the per-user app folder under APPDATA. Theme, motion, zoom, panes, the recent project list and update checks all keep their choice here. Losing a preference must never stop the app, so reads and writes never raise errors.

## Source

- `crates/codewiki-app/src/prefs.rs` — module `codewiki_app::prefs`

## How it works

`dir` builds the folder from the APPDATA variable and the app folder name from `cw_core::paths`. `read`, `write` and `remove` each call `dir` and then a private twin (`read_in`, `write_in`, `remove_in`) that takes the folder as an argument. The twins exist so tests can use a temporary folder.

`write_in` creates the folder if needed, then writes the file name plus .txt. `read_in` trims the text and treats an empty result as not set. `remove_in` deletes the file.

## Items

- `pub fn dir() -> Option<PathBuf>` — the settings directory, or `None` when the system does not say where application data goes.
- `pub fn read(name: &str) -> Option<String>` — the value of the setting `name`, trimmed. `None` when it is not set or is empty.
- `pub fn write(name: &str, value: &str)` — remember `value` for the setting `name`.
- `pub fn remove(name: &str)` — forget the setting `name`. A setting that was never written is already forgotten.
- `fn file(dir: &Path, name: &str) -> PathBuf` — private function, returns `PathBuf`; called by `read_in`, `remove_in`, `write_in`.
- `fn read_in(dir: &Path, name: &str) -> Option<String>` — private function, returns `Option<String>`; called by `read`.
- `fn write_in(dir: &Path, name: &str, value: &str)` — private function; called by `write`.
- `fn remove_in(dir: &Path, name: &str)` — private function; called by `remove`.

## Depends on

- [[Paths Module]] — uses `APP_FOLDER`

## Used by

- [[Startup Module]] — from `keep_recent`, `open_last`, `recent`, `remember`
- [[Update Module]] — from `auto_check`, `check`, `due`, `set_auto_check`
- [[codewiki-app]] — from `after_hide`, `main`, `run_settings_action`
- [[Tab Prefs Module]] — from `init`, `set_new_tab`, `set_tabs`
- [[Themes Module]] — from `init`, `select`, `user_theme_dir`
- [[Backdrop Module]] — from `init`, `set_shown`
- [[Motion Module]] — from `init`, `set`
- [[Panes Module]] — from `load`, `store`

## Gotchas

- If APPDATA is not set, `dir` gives nothing. Then reads return not set and writes do nothing, silently.
- Write errors are thrown away on purpose. A setting can fail to save with no sign.
- A blank value reads back as not set, so an empty string cannot be told from a removed setting.
- Values are trimmed on read, so leading and trailing spaces are lost.
- A development build uses a different folder name, so its settings do not mix with an installed copy's.

## See also

- [[_index]]
- [[codewiki-app]]
