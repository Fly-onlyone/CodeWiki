---
tags: [core, cw-server, module]
---

# Claims Module

> Which waiting notes an agent holds.

## What it does

Several coding-agent sessions may explain notes at once. The backend lends each note it lists to one agent for `cw_core::CLAIM_MINUTES` minutes, so that two agents never get the same one. Nothing is written to disk: a new backend starts with no claims, and an expired one frees itself.

## Source

- `crates/cw-server/src/claims.rs` — module `cw_server::claims`

## How it works

1. `Claims::claim` and `Claims::renew` in [[Claims]] call `period`.

## Items

- [[Claims]] — struct
- `fn period() -> Duration` — private function, returns `Duration`; called by `Claims::claim`, `Claims::renew`.

## Depends on

- [[cw-core]] — uses `CLAIM_MINUTES`

## Used by

- [[Claims]] — from `claim`, `renew`

## See also

- [[_index]]
- [[cw-server]]
