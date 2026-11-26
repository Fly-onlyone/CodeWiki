---
tags: [core, cw-server]
---

# Activity

> `Activity` struct defined in `cw_server::activity` with 4 methods.

## What it does

`Activity` is the backend's in-memory status board. Other parts of the server tell it when a bridge calls, when notes change, and when an update starts. `GET /v1/status` reads it through `report`.

## Source

- `crates/cw-server/src/activity.rs` — definition of `Activity`

## How it works

Three kinds of state sit inside. `last_bridge_call` is an atomic time stamp, set by `bridge_called`. `notes` is an atomic counter that `notes_changed` raises by one. A client that sees the number move asks for the notes again. `updates` is a mutex-guarded list, filled by `begin_update`, which returns a guard. Dropping the guard removes the entry. `report` turns all of it into JSON with the version, the current time, the running updates, the last bridge call and `notes_rev`. See [[Activity Module]] for the guard and the clock helper.

## Items

Declared as `pub(crate) struct Activity`.

Members:

- `pub fn bridge_called(&self)` — record that an agent's MCP bridge called just now.
- `pub fn notes_changed(&self)` — record that the notes that wait changed: a note was written, claimed, freed or reviewed, or an update ran.
- `pub fn begin_update(&self, repo: &str) -> UpdateGuard<'_>` — record an update of `repo` as running, from now until the guard is dropped. An update that waits for another one counts as running.
- `pub fn report(&self) -> Value` — the body of `GET /v1/status`. `now` is this machine's clock, so that a client works with ages and its own clock does not matter.

## Depends on

- [[Activity Module]] — uses `RunningUpdate`, `UpdateGuard`, `now`
- [[cw-core]] — uses `VERSION`

## Used by

- [[Api Module]] — from `approve_proposal`, `backend_status`, `list_explanations`, `reject_proposal`
- [[Activity Module]] — from `UpdateGuard`
- [[Inner]] — from `Inner`

## Gotchas

- Counters use relaxed atomic ordering. They are hints for clients, not a lock.
- `report` sends `now` so clients compute ages from the server clock, not their own.
- State is lost on restart.

## See also

- [[_index]]
- [[Activity Module]]
