---
tags: [core, codewiki-app, module]
---

# Tab Prefs Module

> The two tab settings: whether the app has tabs at all, and whether a note the user opens goes to a tab of its own.

## What it does

This module holds the two tab settings. One says whether the app has tabs at all. The other says whether a note the user opens goes to a tab of its own. Each is a small file in the settings folder, read through the [[Prefs Module]]. The module also keeps both as a GPUI global, so a click handler can read them without touching a file.

## Source

- `crates/codewiki-app/src/tab_prefs.rs` — module `codewiki_app::tab_prefs`

## How it works

`main` calls `init` once at start. `init` reads the two files and stores a `Settings` global. A file holds the word `on` or `off`. Tabs are on unless the file says `off`. New-tab opening is off unless the file says `on`.

`tabs`, `new_tab_switch` and `new_tab` read the global. `tabs` says whether the app shows tabs. `new_tab_switch` is the switch exactly as the user set it. `new_tab` is true only when the switch is on and tabs are on too, so it is the one `open_note_for_user` should ask.

`set_tabs` and `set_new_tab` write the file first, then replace the global with the new value. Changes from the settings page therefore last across restarts and take effect at once.

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

## Gotchas

- A word the module does not know keeps the default. `maybe` for tabs means tabs on, and `yes` for new tab means off. A test shows this.
- With tabs off, the switch can still read `on`. `new_tab_switch` shows that, while `new_tab` says false. Use `new_tab` to decide where a note opens.
- `init` must run before the readers. If the global is missing, the readers fall back to the defaults, so a missed `init` gives defaults, not an error.
- Only a write that goes through `set_tabs` or `set_new_tab` updates the global. Editing the file by hand changes nothing until the next start.

## See also

- [[_index]]
- [[codewiki-app]]
