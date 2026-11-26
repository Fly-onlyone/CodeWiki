---
tags: [core, codewiki-app, module]
---

# Tab Prefs Module

> The two tab settings: whether the app has tabs at all, and whether a note the user opens goes to a tab of its own.

## What it does

Each is a file in the settings folder (`prefs`) and a global, so that a click reads it without a file.

## Source

- `crates/codewiki-app/src/tab_prefs.rs` — module `codewiki_app::tab_prefs`

## How it works

1. `main` in [[codewiki-app]] calls `init`. `init` calls `Settings::from`, then `read` in [[Prefs Module]].
2. `AppView::body` and `AppView::context_items` in [[codewiki-app]], and 5 more, call `tabs`. `tabs` calls `settings`. `settings` calls `Settings::from`.
3. `AppView::settings_frame` in [[codewiki-app]] calls `new_tab_switch`. `new_tab_switch` calls `settings`.
4. `AppView::open_note_for_user` in [[codewiki-app]] calls `new_tab`. `new_tab` calls `settings`.
5. `AppView::run_action_with` in [[codewiki-app]] calls `set_tabs`. `set_tabs` calls `write` in [[Prefs Module]], then `word`, then `settings`.

1 more entry point: `set_new_tab`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const TABS: &str` — the setting that turns tabs on and off.
- `const NEW_TAB: &str` — the setting that opens every note in a new tab.
- `pub fn init(cx: &mut App)` — read the settings. Call once at start.
- `fn settings(cx: &App) -> Settings` — private function, returns `Settings`; called by `new_tab`, `new_tab_switch`, `set_new_tab`, `set_tabs` and 1 more.
- `pub fn tabs(cx: &App) -> bool` — whether the app shows tabs. Off, it has one view, and every note opens in it.
- `pub fn new_tab_switch(cx: &App) -> bool` — whether the switch "Open notes in a new tab" is on, whatever `tabs` says.
- `pub fn new_tab(cx: &App) -> bool` — whether a note the user opens goes to a tab of its own: the switch is on and so are tabs.
- `pub fn set_tabs(on: bool, cx: &mut App)` — turn tabs on or off, and remember it.
- `pub fn set_new_tab(on: bool, cx: &mut App)` — turn "Open notes in a new tab" on or off, and remember it.
- `fn word(on: bool) -> &'static str` — private function, returns `&'static str`; called by `set_new_tab`, `set_tabs`.

And 1 more private item with no comment.

## Depends on

- [[Prefs Module]] — uses `read`, `write`

## Used by

- [[codewiki-app]] — from `body`, `context_items`, `main`, `new_tab`

## See also

- [[_index]]
- [[codewiki-app]]
