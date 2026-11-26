---
tags: [core, cw-server, module]
---

# Activity Module

> What the backend is doing right now, for `GET /v1/status`.

## What it does

Keeps a small, in-memory record of what the backend is doing, so `GET /v1/status` can answer. A client such as the app cannot see this otherwise. It shows which wiki updates run now, when an agent's MCP bridge last called, and a counter that tells clients when the list of waiting notes changed.

Nothing is saved to disk. A new backend process starts with an empty record.

## Source

- `crates/cw-server/src/activity.rs` — module `cw_server::activity`

## How it works

[[Activity]] holds the record. Other code calls four methods on it.

- `begin_update` adds a running update for a repository and returns an `UpdateGuard`. While the guard lives, the update shows in the report. When the guard is dropped, its `Drop` code removes exactly that update. It runs on a panic too, so a failed update does not stay listed forever.
- `bridge_called` stores the time of the latest bridge call.
- `notes_changed` adds one to a counter. The report shows it as `notes_rev`. A client that sees the number move asks for the waiting notes again.
- `report` builds the JSON body: the version, the current time, the running updates, the last bridge call, and `notes_rev`.

The report carries `now`, the server's own clock. A client works out ages from it, so its own clock does not matter.

## Items

- `fn now() -> u64` — seconds since the Unix epoch, on this machine's clock.
- [[Activity]] — struct
- `pub(crate) struct UpdateGuard<'a>` — marks an update as running until it is dropped, a panic included.

And 1 more private item with no comment.

## Depends on

- [[Activity]] — uses `Activity`

## Used by

- [[Activity]] — from `Activity`, `begin_update`, `bridge_called`, `report`

## Gotchas

- Each update has its own key. Two updates of one repository can run, or wait, at the same time. Dropping one guard removes only its own entry.
- An update that waits for another one counts as running.
- The bridge time 0 means never, and `report` shows it as null. So `bridge_called` stores at least 1, even on a clock set to the epoch.
- A poisoned lock does not stop the report. The code takes the data out of the lock and goes on.
- The record covers one process. A restart clears it.

## See also

- [[_index]]
- [[cw-server]]
