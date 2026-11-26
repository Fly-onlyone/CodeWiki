---
tags: [core, codewiki-app, module]
---

# Update Module

> Updating the app: when to look for a new release, and what the settings page says about it.

## What it does

This module decides when the app looks for a new release, and what the Version row of the settings page says about it. The download and install work is in [[cw-update]]. Here the app only keeps the state of the update, the user's on/off choice for the daily check, and the time of the last answer from GitHub.

An installed app checks once a day unless the user turned it off. It checks at start, and because the app stays in the tray for weeks, it also asks every hour whether the day is over.

## Source

- `crates/codewiki-app/src/update.rs` — module `codewiki_app::update`

## How it works

`State` is the one enum the settings page reads: not installed (a development build), idle, checking, up to date, a release available, unavailable with a reason, installing, or installed and waiting for a restart. `State::describe` turns each into the text of the Version row. `State::can_check` says whether a new check may start, and `State::installing` tells the app to start no new work while the backend is stopped.

`due` reads the stored time of the last answer through [[Prefs Module]] and passes it to `due_after`. `due_after` is true when no check ever ran, when a full day has passed, or when the stored time lies in the future.

`check` is blocking. It asks GitHub through `cw_update::check` with the current version. If the answer is anything but `Unavailable`, it stores the time. `State::from_check` turns the answer into a `State`.

`auto_check` and `set_auto_check` read and write the on/off setting. Only the value "off" turns the check off; a missing value means on.

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

## Gotchas

- The time is saved only when GitHub answered. A check with no network leaves the old time, so the next hourly look tries again.
- A stored time in the future (the clock was set back) counts as due. Otherwise the check would wait until the clock caught up.
- `check` blocks. Call it off the UI thread.
- The setting names `update_check` and `update_checked` are stored in prefs; renaming them resets every user's choice.

## See also

- [[_index]]
- [[codewiki-app]]
