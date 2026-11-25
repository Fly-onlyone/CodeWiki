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

1. `State::new` in [[Settings View Module]] and `Supervisor::default` in [[Supervisor]] call `Backoff::new`.
2. `AppView::embed_step` in [[codewiki-app]] calls `embed_step`.
3. `AppView::probe_embed_models` in [[codewiki-app]] calls `embed_came_up`.

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

## See also

- [[_index]]
- [[codewiki-app]]
