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

A `Notice` has a key, a tone, a topic (which gives the toast icon), a title, an optional body, an optional action button and a `throttle` flag.

While the window is on screen, a notice goes to `Toasts::push`: at most three toasts stay (`MAX_SHOWN`), the oldest is pushed out and fades. A notice with the key of one already shown replaces it in place and gets a new sequence number. When the window is hidden or minimized, the notice goes to Windows instead. `system` converts it to a Windows notification whose tag is the key and whose one button, if the notice has an action, has a fixed id.

`Toasts::sent` remembers the last eight notices sent, so `route` can turn a click on a Windows notification (by tag) into the action. Clicks on the button or the body both work; a click on any other button id is ignored.

For notices that can repeat at every status poll, `Toasts::offer` limits Windows to one per key per minute (`SYSTEM_GAP`). A throttled notice that comes too soon waits, and a newer one with the same key replaces it. `take_due` gives back those whose wait is over.

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

## Gotchas

- Expiry uses the sequence number. A timer from a replaced toast must not remove the newer one, so `expire` returns false for it.
- Only notices with `throttle` set are delayed. Others, such as the result of the user's own action, show at once.
- A Topic::App notice only ever goes to Windows, so its icon is never drawn.
- Nothing here draws or touches GPUI. `main.rs` decides where a notice goes; this module only keeps state.
- Only eight sent notices are kept, so a click on a very old Windows notification may do nothing.

## See also

- [[_index]]
- [[codewiki-app]]
