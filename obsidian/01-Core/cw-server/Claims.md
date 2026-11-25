---
tags: [core, cw-server]
---

# Claims

> `Claims` struct defined in `cw_server::claims` with 5 methods.

## Source

- `crates/cw-server/src/claims.rs` — definition of `Claims`

## How it works

What the functions of this type call:

1. `claim` calls `period` in [[Claims Module]].
2. `renew` calls `period` in [[Claims Module]].

## Items

Declared as `pub(crate) struct Claims`.

Members:

- `pub fn expire(&mut self, now: Instant) -> bool` — drop the claims that ran out. True when any went.
- `pub fn others(&self, repo: &str, agent: Option<&str>) -> Vec<String>` — the notes of `repo` held by an agent other than `agent`; with `None`, every held note.
- `pub fn claim(&mut self, repo: &str, agent: &str, notes: &[String], now: Instant) -> bool` — lend `notes` to `agent`. True when anything changed.
- `pub fn renew(&mut self, repo: &str, agent: &str, now: Instant)` — push every claim `agent` holds in `repo` to a full period from now.
- `pub fn release(&mut self, repo: &str, note: &str) -> bool` — free one note. True when it was held.

## Depends on

- [[Claims Module]] — uses `period`

## Used by

- [[Api Module]] — from `backend_status`, `list_explanations`, `lock_claims`, `write_explanation`
- [[Inner]] — from `Inner`

## See also

- [[_index]]
- [[Claims Module]]
