---
tags: [core, cw-server, module]
---

# Claims Module

> Which waiting notes an agent holds.

## What it does

Several coding agents may explain wiki notes at once. This module lets the backend lend each waiting note to one agent for a fixed time (`CLAIM_MINUTES`), so two agents never write the same note. The state is in memory only. A new backend starts with no claims, and an expired claim frees itself.

## Source

- `crates/cw-server/src/claims.rs` — module `cw_server::claims`

## How it works

[[Claims]] keeps a map from (repository id, note path) to (agent id, deadline). `period` turns `CLAIM_MINUTES` into a `Duration`. `claim` and `renew` use it to set a new deadline. `expire` removes entries whose deadline has passed.

## Items

- [[Claims]] — struct
- `fn period() -> Duration` — private function, returns `Duration`; called by `Claims::claim`, `Claims::renew`.

## Depends on

- [[cw-core]] — uses `CLAIM_MINUTES`

## Used by

- [[Claims]] — from `claim`, `renew`

## Gotchas

- The period is one value for all claims. Renewing pushes every claim of that agent in that repository to a full period from now.
- Claims are not saved. After a restart every note looks free.

## See also

- [[_index]]
- [[cw-server]]
