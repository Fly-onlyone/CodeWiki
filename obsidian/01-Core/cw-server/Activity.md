---
tags: [core, cw-server]
---

# Activity

> `Activity` struct defined in `cw_server::activity` with 3 methods.

## Source

- `crates/cw-server/src/activity.rs` — definition of `Activity`

## How it works

What the functions of this type call:

1. `bridge_called` calls `now` in [[Activity Module]].
2. `begin_update` calls `now` in [[Activity Module]].
3. `report` calls `now` in [[Activity Module]].

## Items

Declared as `pub(crate) struct Activity`.

Members:

- `pub fn bridge_called(&self)` — record that an agent's MCP bridge called just now.
- `pub fn begin_update(&self, repo: &str) -> UpdateGuard<'_>` — record an update of `repo` as running, from now until the guard is dropped. An update that waits for another one counts as running.
- `pub fn report(&self) -> Value` — the body of `GET /v1/status`. `now` is this machine's clock, so that a client works with ages and its own clock does not matter.

## Depends on

- [[Activity Module]] — uses `RunningUpdate`, `UpdateGuard`, `now`
- [[cw-core]] — uses `VERSION`

## Used by

- [[Api Module]] — from `backend_status`, `require_token`, `run_update`
- [[Activity Module]] — from `UpdateGuard`
- [[Inner]] — from `Inner`

## See also

- [[_index]]
- [[Activity Module]]
