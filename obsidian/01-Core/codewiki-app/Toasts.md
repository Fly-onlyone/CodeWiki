---
tags: [core, codewiki-app]
---

# Toasts

> The toasts on screen, the ones that are leaving, and the notices sent to Windows.

## What it does

`Toasts` is the store behind the app's notifications. It holds the toasts on screen, the ones that were just removed and are fading out, the notices handed to Windows, and the notices waiting for their turn. It holds state only; the app draws it. See [[Toasts Module]] for the notice type and the rules.

## Source

- `crates/codewiki-app/src/toasts.rs` — definition of `Toasts`

## How it works

`push` adds a toast and trims the stack to three, sending the oldest to `leave`. `expire` removes by sequence number and `dismiss` by key; both go through `leave`, which keeps the fading list no longer than the shown limit.

For Windows, `offer` decides whether a notice is shown now or waits. It then calls `sent` to remember the notice, so `route` can answer a click on it. A notice marked as repeating waits when one with its key was shown less than a minute ago. `take_due` and `next_due` let the app wake at the right time. `live` says whether a key is still before the user, as a toast, a recent Windows notification, or one waiting. The review code uses it so a repeated notice can add up what came since the first.

## Items

Declared as `pub struct Toasts`.

Members:

- `pub fn push(&mut self, notice: Notice) -> u64` — show a notice and return its sequence number, for `expire`. One with the same key is replaced and the new one is the newest.
- `pub fn expire(&mut self, seq: u64) -> bool` — take off the toast that still has this sequence number. Returns whether there was one: an older toast with the same key may have been replaced since, and its timer must not remove the newer one.
- `pub fn dismiss(&mut self, key: &str) -> bool` — take off the toast with this key. Returns whether there was one.
- `fn leave(&mut self, toast: Toast)` — private method; called by `Toasts::dismiss`, `Toasts::expire`, `Toasts::push`.
- `pub fn shown(&self) -> &[Toast]` — the toasts on screen, oldest first.
- `pub fn leaving(&self) -> &[Toast]` — the toasts that were just taken off, which are still fading out.
- `pub fn sent(&mut self, notice: &Notice)` — remember a notice that goes to Windows, so that a click on it can be answered by `Toasts::route`.
- `pub fn route(&self, tag: &str, action_id: Option<&str>) -> Option<Action>` — what a click on a Windows notification does: the action of the notice with this tag, for a click on its button or on its body. `None` for a notice with no action, and for one this app does not know.
- `pub fn offer(&mut self, notice: Notice, now: Instant) -> Offer` — offer a notice to Windows at `now`: it is shown at once, unless it is one that repeats (`Notice::throttle`) and one with its key was shown less than `SYSTEM_GAP` ago; then it waits.
- `pub fn take_due(&mut self, now: Instant) -> Vec<Notice>` — the notices whose wait is over at `now`, in the order they began to wait. Each is told again, as a toast or to Windows.
- `pub fn next_due(&self, now: Instant) -> Option<Duration>` — how long until the next waiting notice is due; `None` when none waits.
- `pub fn live(&self, key: &str, now: Instant) -> bool` — whether a notice with this key is still before the user: a toast on screen, or a Windows notification shown in the last `SYSTEM_GAP` or waiting to be.

## Depends on

- [[Toasts Module]] — uses `ACTION_ID`, `MAX_SENT`, `MAX_SHOWN`, `Notice`
- [[Review Module]] — uses `retain`
- [[Action]] — uses `Action`

## Used by

- [[codewiki-app]] — from `AppView`, `dismiss_toast`, `handle_event`, `notify_user`

## Gotchas

- `push` also cancels a waiting Windows notice with the same key. Otherwise it would be shown later and replace the newer toast.
- Window state of the key is forgotten after the gap: `offer` drops old entries each time it runs.
- `leaving` holds copies for the fade-out only. Never treat them as still shown.

## See also

- [[_index]]
- [[Toasts Module]]
