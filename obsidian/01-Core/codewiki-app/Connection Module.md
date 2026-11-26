---
tags: [core, codewiki-app, module]
---

# Connection Module

> When the app tries to reach its backend again, and how.

## What it does

Nothing here touches GPUI or the network. After each status poll `main.rs` hands `Supervisor` what the poll found, asks it whether an attempt is due, runs that attempt in a task of its own and hands the result back. A sequence number drops the result of an attempt that the user overtook by switching backends or pressing "Try again".

## Source

- `crates/codewiki-app/src/connection.rs` — module `codewiki_app::connection`

## How it works

The state lives in [[Supervisor]]. After each status poll the app calls `polled` with `Answered`, `Silent` or `NotAsked`. `SILENT_POLLS` (2) silent polls in a row make the backend lost, and the supervisor records a `Loss` with a due time.

The app asks `due`. It returns an `Attempt` and the current sequence number when the wait is over. The first attempt of a loss on this machine is `Full`, which may start a backend. Later ones are `ProbeOnly`, which only look for one that answers. A named remote backend is never started.

The attempt runs in its own task. Its result comes back through `failed`, which schedules the next try after `Backoff` (5 seconds, doubling to 30), or through `succeeded`, which clears the loss and says what was lost.

`Sequence` guards against old results. `restart` (the user switched backends or pressed "Try again") and `retry_now` bump the number, and a result with an older number is dropped.

Separately, `embed_step` and `embed_came_up` decide about the embedding server: retry only on its settings page, with a local backend, while the window is on screen and the last answer was "not reachable". A minimized window waits one second and looks again.

## Items

- `pub const BACKEND_FIRST: Duration` — the first wait before the backend is tried again, and the longest.
- `pub const BACKEND_CAP: Duration` — public constant; used by `Supervisor::default`.
- `pub const SILENT_POLLS: u32` — status polls in a row that must go unanswered before the backend counts as lost. One alone can fail on a busy machine.
- `pub const EMBED_FIRST: Duration` — the first wait before the embedding server is asked again, and the longest.
- `pub const EMBED_CAP: Duration` — public constant; used by `State::new`.
- `pub const EMBED_IDLE: Duration` — how long to wait before looking again while the window is minimized.
- `pub enum EmbedStep` — what the retry of the embedding server does next.
- `embed_step` — whether to ask the embedding server again. It is retried only on its settings page, with a backend on this machine, while the window is on screen and the last answer said the server is not reachable.
- `pub fn embed_came_up(before: Option<(&str, bool)>, after: (&str, bool)) -> bool` — whether an answer says the server came up: it was not reachable and now is, for the same provider.
- `pub struct Backoff` — waits that double from a first one up to a cap.
- `pub struct Sequence` — numbers the attempts. An attempt takes the number current when it starts; its result counts only while that number is still current, so a bump drops every result of the attempts made before it.
- `pub enum Attempt` — what an attempt may do.
- `pub enum Lost` — when the backend was lost.
- `pub enum Poll` — what a status poll of the backend in use found.
- `struct Loss` — A backend that is lost, and the attempts to reach one again.
- [[Supervisor]] — watches the backend and decides when, and how, to try it again.

## Used by

- [[Supervisor]] — from `Supervisor`, `default`, `due`, `failed`
- [[codewiki-app]] — from `embed_step`, `load_embed_models`, `probe_embed_models`, `reach`
- [[Settings View Module]] — from `State`, `new`
- [[Doc View Module]] — from `show`

## Gotchas

- Nothing here uses GPUI, the clock or the network directly. Callers pass `Instant` values in, so tests need no waiting.
- A late result of an overtaken attempt must be checked with `is_current` before use. `failed` returns false and `succeeded` returns `None` for such results.
- A poll that answers while a loss is open makes the attempt due now. This covers a remote backend that comes back at the same address and token.
- `lose` is for a backend that is gone without a poll saying so, as after a failed update. On a loss already open it only lets the next attempt start a local backend again.
- `retry_now` returns false for the offline page after startup. That page's "Try again" is a new connect through `restart`.
- `Backoff::new` caps the first wait at the cap, so a first wait above the cap is never used.

## See also

- [[_index]]
- [[codewiki-app]]
