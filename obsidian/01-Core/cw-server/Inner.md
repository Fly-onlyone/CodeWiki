---
tags: [core, cw-server]
---

# Inner

> `Inner` struct defined in `cw_server`.

## What it does

`Inner` is the shared state of the whole backend: the config and every lock, counter and job that handlers must share. One `Inner` exists per server. Handlers reach it through [[AppState]].

## Source

- `crates/cw-server/src/lib.rs` — definition of `Inner`

## How it works

`state` builds one `Inner` from a `ServerConfig` and wraps it in an `Arc`. Fields:

- `write_lock` serializes writers: repository list changes, note writes and wiki updates.
- `settings_lock` serializes settings changes. It is separate because an update holds `write_lock` for minutes.
- `stop` is woken by `POST /v1/shutdown`, and `serve` then ends.
- `activity` is the live status board, see [[Activity]].
- `claims` lends waiting notes to agents, see [[Claims]].
- `jobs` is the background embedding job, see [[Jobs (cw-server)]].
- `proposals` caches the pending proposal counts, see [[Pending Module]].
- `updates` counts finished wiki updates. A background embedding that sees it change starts its repository over.

## Items

Declared as `pub(crate) struct Inner`.

## Depends on

- [[cw-server]] — uses `ServerConfig`
- [[Pending Module]] — uses `ProposalCounts`
- [[Activity]] — uses `Activity`
- [[Claims]] — uses `Claims`
- [[Jobs (cw-server)]] — uses `Jobs`

## Used by

- [[cw-server]] — from `state`
- [[AppState]] — from `AppState`

## Gotchas

- Lock order: take `write_lock` first and `claims` inside it, briefly. Never the other way round, or two requests can deadlock.
- Do not put settings changes under `write_lock`. A setting would then wait for a long update.

## See also

- [[_index]]
- [[cw-server]]
