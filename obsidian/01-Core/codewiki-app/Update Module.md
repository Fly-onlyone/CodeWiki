---
tags: [core, codewiki-app, module]
---

# Update Module

> Updating the app: when to look for a new release, and what the settings page says about it.

## Source

- `crates/codewiki-app/src/update.rs` — module `codewiki_app::update`

## How it works

The work itself is in `cw-update`.

Main items:

- `State` — where the app stands with updates.
- `check` — ask GitHub, and remember when it answered.
- `auto_check` — whether the app checks by itself when it starts.
- `set_auto_check` — function
- `due` — whether the daily check is due.
- `now` — function
- `AUTO_CHECK` — the setting that turns the daily check off ("off") or on.
- `INTERVAL` — const

## Depends on

- [[Prefs Module]] — uses `read`, `write`
- [[Release Module]] — uses `Check`, `check`
- [[cw-core]] — uses `VERSION`
- [[cw-update]] — uses `api_base`
- [[Release]] — uses `Release`

## Used by

- [[codewiki-app]] — from `AppView`, `backend_to_stop`, `check_for_update`, `home_doc`
- [[Pages Module]] — from `settings_page`, `updates_card`

## See also

- [[_index]]
- [[codewiki-app]]
