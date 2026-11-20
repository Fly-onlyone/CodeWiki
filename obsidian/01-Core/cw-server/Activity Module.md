---
tags: [core, cw-server, module]
---

# Activity Module

> What the backend is doing right now, for `GET /v1/status`.

## What it does

Two things a client cannot see otherwise: which wiki updates are running, whoever started them (the app, an agent's bridge, a git webhook), and when an agent's MCP bridge last called. Nothing here is written to disk: it describes this process, and a new backend starts with none of it.

## Source

- `crates/cw-server/src/activity.rs` — module `cw_server::activity`

## How it works

1. `Activity::begin_update` and `Activity::bridge_called` in [[Activity]], and 1 more, call `now`.

## Items

- `fn now() -> u64` — seconds since the Unix epoch, on this machine's clock.
- [[Activity]] — struct
- `pub(crate) struct UpdateGuard<'a>` — marks an update as running until it is dropped, a panic included.

And 1 more private item with no comment.

## Depends on

- [[Activity]] — uses `Activity`

## Used by

- [[Activity]] — from `Activity`, `begin_update`, `bridge_called`, `report`

## See also

- [[_index]]
- [[cw-server]]
