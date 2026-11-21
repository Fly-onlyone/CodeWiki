---
tags: [core, codewiki-app, module]
---

# Prefs Module

> Small per-user settings, one value per file under `%APPDATA%\CodeWiki`.

## What it does

Each setting is a name and a text, mostly one line; the recent list puts one folder on each line. A missing or unreadable file is "not set"; a failed write is ignored, because losing a preference must never stop the app.

## Source

- `crates/codewiki-app/src/prefs.rs` — module `codewiki_app::prefs`

## How it works

1. `AppView::after_hide` and `main` in [[codewiki-app]], and 9 more, call `read`. `read` calls `read_in`, then `dir`. `read_in` calls `file`.
2. `AppView::after_hide` and `AppView::run_settings_action` in [[codewiki-app]], and 10 more, call `write`. `write` calls `dir`, then `write_in`. `write_in` calls `file`.
3. `acquire` in [[Instance Module]] and `user_theme_dir` in [[Themes Module]] call `dir`.
4. `State::new` in [[Settings View Module]] calls `remove`. `remove` calls `dir`, then `remove_in`. `remove_in` calls `file`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub fn dir() -> Option<PathBuf>` — the settings directory, or `None` when the system does not say where application data goes.
- `pub fn read(name: &str) -> Option<String>` — the value of the setting `name`, trimmed. `None` when it is not set or is empty.
- `pub fn write(name: &str, value: &str)` — remember `value` for the setting `name`.
- `pub fn remove(name: &str)` — forget the setting `name`. A setting that was never written is already forgotten.
- `fn file(dir: &Path, name: &str) -> PathBuf` — private function, returns `PathBuf`; called by `read_in`, `remove_in`, `write_in`.
- `fn read_in(dir: &Path, name: &str) -> Option<String>` — private function, returns `Option<String>`; called by `read`.
- `fn write_in(dir: &Path, name: &str, value: &str)` — private function; called by `write`.
- `fn remove_in(dir: &Path, name: &str)` — private function; called by `remove`.

## Used by

- [[Startup Module]] — from `keep_recent`, `open_last`, `recent`, `remember`
- [[Update Module]] — from `auto_check`, `check`, `due`, `set_auto_check`
- [[codewiki-app]] — from `after_hide`, `main`, `run_settings_action`
- [[Themes Module]] — from `init`, `select`, `user_theme_dir`
- [[Motion Module]] — from `init`, `set`
- [[Panes Module]] — from `load`, `store`
- [[Zoom Module]] — from `init`, `set`
- [[Instance Module]] — from `acquire`

## See also

- [[_index]]
- [[codewiki-app]]
