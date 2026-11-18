---
tags: [core, cw-server, module]
---

# Activity Module

> What the backend is doing right now, for `GET /v1/status`.

## Source

- `crates/cw-server/src/activity.rs` — module `cw_server::activity`

## How it works

Main items:

- [[Activity]] — struct
- `RunningUpdate` — struct
- `now` — seconds since the Unix epoch, on this machine's clock.
- `UpdateGuard` — marks an update as running until it is dropped, a panic included.

## Depends on

- [[Activity]] — uses `Activity`

## Used by

- [[Activity]] — from `Activity`, `begin_update`, `bridge_called`, `report`

## See also

- [[_index]]
- [[cw-server]]
