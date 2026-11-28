---
tags: [core, cw-server]
---

# AppState

> `AppState` struct defined in `cw_server`.

## What it does

The shared state of the HTTP server. It is a clonable handle to one `Inner` value. Every route handler gets a clone, so all requests see the same config, locks and counters. The background embedding job and the settings code take the same handle.

## Source

- `crates/cw-server/src/lib.rs` — definition of `AppState`

## How it works

`AppState` wraps an `Arc` of [[Inner]], so a clone is cheap and points at the same data. `state` builds it from a `ServerConfig` when the server starts. `router` and `serve` both call it.

`Inner` holds:

- the config;
- `write_lock`, which serializes repository list changes and vault updates;
- `settings_lock`, a second lock for settings changes;
- `stop`, a `Notify` that `POST /v1/shutdown` fires;
- `activity`, for `GET /v1/status`;
- `claims`, the waiting notes lent to agents;
- `jobs`, the background embedding job;
- `proposals`, the count of pending proposals per repository;
- `updates`, a counter of finished wiki updates.

`serve` binds the port, publishes it through `on_bound`, and applies the settings to semantic search. Then, on a blocking task, it takes `write_lock` and tidies the repository list: it marks linked worktrees and forgets those whose folder is gone. After that it calls `embed_job::resume`. When the stop signal comes, from the caller or from `POST /v1/shutdown`, it calls `jobs.close()` so a background job does not keep the process alive.

## Items

Declared as `pub(crate) struct AppState`.

## Depends on

- [[Inner]] — uses `Inner`

## Used by

- [[Api Module]] — from `add_repo`, `approve_proposal`, `ask`, `backend_status`
- [[Embed Job Module]] — from `after_update`, `embed_all`, `embed_repo`, `resume`
- [[Settings Module (cw-server)]] — from `change`, `describe`, `embed_models`, `resolved`
- [[cw-server]] — from `state`

## Gotchas

- Lock order matters. Take `write_lock` first and `claims` inside it, held briefly. Nothing may take them the other way round.
- `settings_lock` is separate on purpose. An update holds `write_lock` for minutes, and a settings change must not wait for it.
- `updates` lets a running background embedding notice that a wiki update finished, so it starts its repository over.
- `router` builds a fresh state each call. Tests use it to drive the API without a socket.

## See also

- [[_index]]
- [[cw-server]]
