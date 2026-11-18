---
tags: [core, cw-server]
---

# Activity

> `Activity` struct defined in `cw_server::activity` with 3 methods.

## Source

- `crates/cw-server/src/activity.rs` — definition of `Activity`

## How it works

Declared as `pub(crate) struct Activity`.

Public API:

- `pub fn report(&self) -> Value` — the body of `GET /v1/status`.
- `pub fn bridge_called(&self)` — record that an agent's MCP bridge called just now.
- `pub fn begin_update(&self, repo: &str) -> UpdateGuard<'_>` — record an update of `repo` as running, from now until the guard is dropped.

## Depends on

- [[Activity Module]] — uses `RunningUpdate`, `UpdateGuard`, `now`
- [[cw-core]] — uses `VERSION`
- [[Diagrams Module]] — uses `store`

## Used by

- [[Api Module]] — from `backend_status`, `require_token`, `run_update`
- [[Activity Module]] — from `UpdateGuard`
- [[Inner]] — from `Inner`

## See also

- [[_index]]
- [[Activity Module]]
