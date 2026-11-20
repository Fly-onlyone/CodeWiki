---
tags: [core, codewiki-app, module]
---

# Prefs Module

> Small per-user settings, one value per file under `%APPDATA%\CodeWiki`.

## Source

- `crates/codewiki-app/src/prefs.rs` — module `codewiki_app::prefs`

## How it works

Main items:

- `dir` — the settings directory, or `None` when the system does not say where application data goes.
- `write` — remember `value` for the setting `name`.
- `read` — the value of the setting `name`, trimmed.
- `file` — function
- `write_in` — function
- `read_in` — function

## Used by

- [[Startup Module]] — from `keep_recent`, `open_last`, `recent`, `remember`
- [[Update Module]] — from `auto_check`, `check`, `due`, `set_auto_check`
- [[codewiki-app]] — from `after_hide`, `main`, `run_settings_action`
- [[Themes Module]] — from `init`, `select`, `user_theme_dir`
- [[Motion Module]] — from `init`, `set`
- [[Panes Module]] — from `load`, `store`
- [[Instance Module]] — from `acquire`
- [[Settings View Module]] — from `new`

## See also

- [[_index]]
- [[codewiki-app]]
