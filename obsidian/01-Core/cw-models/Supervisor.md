---
tags: [core, cw-models]
---

# Supervisor

> The parent's side: one child at a time, started when needed.

## Source

- `crates/cw-models/src/worker.rs` — definition of `Supervisor`

## How it works

Declared as `pub struct Supervisor`.

Public API:

- `pub fn status(&self) -> Status` — which child runs, and whether the graphics card was given up.
- `pub fn tick(&self) -> bool` — stop the child if it has been idle for long, or at once while a game runs.
- `pub fn embed(&self, spec: &Spec, kind: Kind, texts: &[String]) -> Result<Vec<Vec<f32>>>` — embed `texts` with the child `spec` names, starting it when it is not running.
- `pub fn stop(&self)` — stop the child now.
- `pub fn forgive_cuda(&self)` — trust the graphics card again, as after a change of the settings.
- `pub fn new(launcher: Box<dyn Launcher>, game: Box<dyn Fn() -> bool + Send + Sync>) -> Self`

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
