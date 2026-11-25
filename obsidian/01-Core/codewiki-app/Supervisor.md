---
tags: [core, codewiki-app]
---

# Supervisor

> Watches the backend and decides when, and how, to try it again.

## Source

- `crates/codewiki-app/src/connection.rs` — definition of `Supervisor`

## How it works

1. `AppView::new` in [[codewiki-app]] calls `default`. `default` calls `Backoff::new` in [[Connection Module]].

## Items

Declared as `pub struct Supervisor`.

Members:

- `fn default() -> Self` — public method, returns `Self`; called by `AppView::new`.
- `pub fn seq(&self) -> u64` — the number of an attempt that starts now, such as the user's connect.
- `pub fn is_current(&self, seq: u64) -> bool` — whether the result of attempt `seq` still counts.
- `pub fn lost(&self) -> Option<Lost>` — what is lost, while it is.
- `pub fn restart(&mut self) -> u64` — the user switched backends or pressed "Try again", and a connect follows. The results of the attempts made before are dropped, and the waits start again from the first. Returns the connect's number.
- `pub fn offline(&mut self, now: Instant)` — the connect failed and the offline page shows. Look for a backend after each wait from now on; these attempts start none.
- `pub fn polled(&mut self, poll: Poll, remote: bool, now: Instant) -> bool` — take what a status poll of the backend in use found. Returns true when this poll makes the backend lost. `remote` says whether that backend was named rather than found or started on this machine; only one on this machine may be started again.
- `pub fn retry_now(&mut self, now: Instant) -> bool` — the user asked to reconnect while the backend in use is lost: drop the attempt under way, and make the next one now, as the first of a loss. Returns false when nothing is lost mid-session; the offline page's "Try again" is a connect of its own (`restart`).
- `pub fn lose(&mut self, remote: bool, now: Instant)` — the backend in use is gone and could not be started again, as after a failed update: lost mid-session, though no poll said so. It was just tried, so the first attempt waits. A loss under way goes on, its next attempt allowed to start a backend on this machine again: the update may have refused the one it had.
- `pub fn due(&mut self, now: Instant) -> Option<(Attempt, u64)>` — the attempt to make now, with its number, when one is due. It runs until `failed` or `succeeded` is given its number, and no other is due meanwhile.
- `pub fn until_due(&self, now: Instant) -> Option<Duration>` — how long until the next attempt is due. `None` while none waits.
- `pub fn failed(&mut self, seq: u64, now: Instant) -> bool` — attempt `seq` reached no backend: try again after the next wait. Returns false for an attempt whose result no longer counts.
- `pub fn succeeded(&mut self, seq: u64) -> Option<Lost>` — attempt `seq` reached a backend. Returns what had been lost, or `None` when the result no longer counts and the backend it found is dropped.

## Depends on

- [[Connection Module]] — uses `Attempt`, `BACKEND_CAP`, `BACKEND_FIRST`, `Backoff`

## Used by

- [[codewiki-app]] — from `AppView`, `connect`, `new`, `palette_matches`

## See also

- [[_index]]
- [[Connection Module]]
