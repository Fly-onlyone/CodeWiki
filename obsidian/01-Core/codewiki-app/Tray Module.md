---
tags: [core, codewiki-app, module]
---

# Tray Module

> The app's icon in the notification area, with its tooltip and its menu.

## Source

- `crates/codewiki-app/src/tray.rs` — module `codewiki_app::tray`

## How it works

Main items:

- `Command` — what a row of the menu asks the app to do.
- `Tray` — the icon, for as long as it lives; dropping it takes the icon away.
- `Snapshot` — what the tray shows of the app, at one moment.
- `Row` — one row of the menu.
- `rows` — the menu for `snapshot`, top to bottom.
- `tooltip` — the tooltip for `snapshot`.
- `RECENT_ROWS` — how many repositories "Recent repositories" lists.
- `menu_text` — `text` as a menu shows it: Windows reads `&` as "underline the next letter", so one that is meant is doubled.

## Depends on

- [[cw-core]] — uses `VERSION`
- [[Status Module]] — uses `tooltip`
- [[Win32 Module]] — uses `keep_open`

## Used by

- [[codewiki-app]] — from `AppView`, `Event`, `run_tray_command`, `start_tray`

## See also

- [[_index]]
- [[codewiki-app]]
