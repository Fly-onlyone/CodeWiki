---
tags: [core, codewiki-app]
---

# Supervisor

> Watches the backend and decides when, and how, to try it again.

## What it does

`Supervisor` decides when the app should try to reach its backend again after losing it, and whether the try may start a backend. It touches no window and no network: the app feeds it what each status poll found, asks if an attempt is due, runs that attempt itself, and reports the result back. That keeps the rules testable.

## Source

- `crates/codewiki-app/src/connection.rs` — definition of `Supervisor`

## How it works

After each poll the app calls `polled`. Two silent polls in a row (`SILENT_POLLS`) make the backend lost mid-session, and the first attempt is due at once. A local backend gets one full attempt (`Attempt::Full`, which may start a backend); every later one only looks (`Attempt::ProbeOnly`). A remote backend is only looked for, never started.

`due` hands out the next attempt with its sequence number, and no other becomes due until `failed` or `succeeded` is called with that number. A failure schedules the next try after a doubling wait, 5 seconds up to 30 (`Backoff`). A success clears the loss and resets the waits.

The sequence number guards against old results. `restart` (the user switched backend or pressed Try again) bumps it, so earlier attempts are dropped when they return. `retry_now` does the same for a mid-session loss and makes the next attempt immediate. `offline` is for a failed first connect: the offline page shows and attempts only look. `lose` covers a backend that vanished with no poll, such as after a failed update.

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

## Gotchas

- A single silent poll never counts: a busy machine can miss one.
- `failed` and `succeeded` ignore a result whose number is no longer current, and a found backend is dropped then.
- While a loss is open, an answered poll makes the next attempt due now. A remote backend that comes back needs no wait.
- `retry_now` returns false for a startup loss; the offline page's Try again is a new connect through `restart`.

## See also

- [[_index]]
- [[Connection Module]]
