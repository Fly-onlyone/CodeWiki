---
tags: [core, codewiki-app, module]
---

# Update Module

> Updating the app: when to look for a new release, and what the settings page says about it.

## What it does

The work itself is in `cw-update`.

An installed app looks once a day unless the user turned that off: when it starts, and, since it stays in the tray for weeks, every hour it looks whether the day is over. The check's time is kept only when GitHub gave an answer, so a check without network is tried again within the hour.

## Source

- `crates/codewiki-app/src/update.rs` — module `codewiki_app::update`

## How it works

1. `AppView::show_check` in [[codewiki-app]] calls `State::from_check`.
2. `AppView::start_updates` and `AppView::settings_frame` in [[codewiki-app]] call `auto_check`. `auto_check` calls `read` in [[Prefs Module]].
3. `AppView::run_action` in [[codewiki-app]] calls `set_auto_check`. `set_auto_check` calls `write` in [[Prefs Module]].
4. `AppView::start_updates` in [[codewiki-app]] calls `due`. `due` calls `due_after`, then `read` in [[Prefs Module]], then `now`.
5. `AppView::check_for_update` and `AppView::start_updates` in [[codewiki-app]] call `check`. `check` calls `check` in [[Release Module]], then `api_base` in [[cw-update]], then `write` in [[Prefs Module]], then `now`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const AUTO_CHECK: &str` — the setting that turns the daily check off ("off") or on.
- `const CHECKED: &str` — the setting that holds when GitHub last answered a check, in seconds since 1970.
- `pub enum State` — where the app stands with updates.
- `pub fn auto_check() -> bool` — whether the app checks by itself when it starts.
- `pub fn set_auto_check(on: bool)` — public function; called by `AppView::run_action`.
- `fn now() -> u64` — private function, returns `u64`; called by `check`, `due`, `due_after`.
- `pub fn due() -> bool` — whether the daily check is due.
- `fn due_after(last: Option<u64>, now: u64) -> bool` — due when it never ran, when a day has passed, or when the stored time is in the future (the clock was set back).
- `pub fn check() -> Check` — ask GitHub, and remember when it answered. Blocking.

And 1 more private item with no comment.

## Depends on

- [[Prefs Module]] — uses `read`, `write`
- [[Release Module]] — uses `Check`, `check`
- [[cw-core]] — uses `VERSION`
- [[cw-update]] — uses `api_base`
- [[Release]] — uses `Release`

## Used by

- [[codewiki-app]] — from `AppView`, `backend_to_stop`, `check_for_update`, `home_doc`
- [[Settings Pages Module]] — from `Model`, `updates`

## See also

- [[_index]]
- [[codewiki-app]]
