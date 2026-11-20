---
tags: [core, cw-models]
---

# Supervisor

> The parent's side: one child at a time, started when needed.

## Source

- `crates/cw-models/src/worker.rs` — definition of `Supervisor`

## How it works

1. `global` in [[Worker Module]] calls `new`.

## Items

Declared as `pub struct Supervisor`.

Members:

- `pub fn new(launcher: Box<dyn Launcher>, game: Box<dyn Fn() -> bool + Send + Sync>) -> Self` — public method, returns `Self`; called by `global`.
- `fn publish(&self, state: &State)` — bring what `Supervisor::status` answers in line with the state. Called wherever the child or the count of failures changes.
- `fn device(state: &State, wanted: Device) -> Device` — the device a request for `wanted` really runs on: the processor once the graphics card failed too often.
- `pub fn embed(&self, spec: &Spec, kind: Kind, texts: &[String]) -> Result<Vec<Vec<f32>>>` — embed `texts` with the child `spec` names, starting it when it is not running. One request at a time: a caller waits for the one before it, so a search slips in between two batches of a long run.
- `pub fn tick(&self) -> bool` — stop the child if it has been idle for long, or at once while a game runs. Never waits for a request under way: it is not idle then. Returns whether a child still runs, so the caller knows when to stop asking.
- `pub fn stop(&self)` — stop the child now. A request under way is let finish first.
- `pub fn forgive_cuda(&self)` — trust the graphics card again, as after a change of the settings.
- `pub fn status(&self) -> Status` — which child runs, and whether the graphics card was given up. Never waits for a request under way.

And 3 more private items with no comment.

## Depends on

- [[Worker Module]] — uses `CUDA_FAILURES`, `Channel`, `Child`, `Hello`
- [[Error]] — uses `external`, `invalid`
- [[Error Module]] — uses `Result`
- [[Logs Module]] — uses `flush`
- [[Device]] — uses `Device`

## Used by

- [[Worker Module]] — from `GLOBAL`, `global`
- [[Store Module (cw-models)]] — from `fetch`

## See also

- [[_index]]
- [[Worker Module]]
