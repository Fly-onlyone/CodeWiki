---
tags: [core, codewiki-app, module]
---

# Toasts Module

> Short messages for the user: what to show and where it goes.

## What it does

A `Notice` is shown as a toast in the window's corner while the window is on screen, and as a Windows notification while it is hidden or minimized. Nothing here draws or touches GPUI's state: `main.rs` decides where a notice goes and draws the stack (`mui::snackbar`), and this module keeps what is shown and what a click on a Windows notification means.

## Source

- `crates/codewiki-app/src/toasts.rs` — module `codewiki_app::toasts`

## How it works

1. `AppView::show_system_notification` in [[codewiki-app]] calls `system`.

## Items

- `pub const MAX_SHOWN: usize` — the most toasts on screen at once. A new one beyond that pushes the oldest out.
- `const MAX_SENT: usize` — the most notices kept to answer a click on a Windows notification.
- `const ACTION_ID: &str` — the id of the one button a Windows notification has.
- `pub const SYSTEM_GAP: Duration` — the least time between two Windows notifications with one key that repeats (`Notice::throttle`), and how long one counts as live. Without it, proposals that keep coming would show a notification every 5 s, at each status poll.
- `pub enum Topic` — what a notice is about. It gives the icon of the notice's toast; a Windows notification has text only.
- `pub struct Notice` — one message for the user.
- `pub struct Toast` — A notice on screen. `seq` tells it apart from an earlier one with the same key.
- `pub enum Offer` — what becomes of a notice offered to Windows (`Toasts::offer`).
- [[Toasts]] — the toasts on screen, the ones that are leaving, and the notices sent to Windows.
- `pub fn system(notice: &Notice) -> SystemNotification` — the notice as a Windows notification.

## Depends on

- [[Action]] — uses `Action`
- [[Glyph]] — uses `Glyph`
- [[Tone]] — uses `Tone`

## Used by

- [[Toasts]] — from `Toasts`, `leave`, `leaving`, `live`
- [[codewiki-app]] — from `after_hide`, `notify_user`, `probe_embed_models`, `show_system_notification`
- [[Review Module]] — from `decided_notices`, `rise_notice`

## See also

- [[_index]]
- [[codewiki-app]]
