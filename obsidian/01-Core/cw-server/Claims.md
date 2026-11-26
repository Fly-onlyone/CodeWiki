---
tags: [core, cw-server]
---

# Claims

> `Claims` struct defined in `cw_server::claims` with 5 methods.

## What it does

`Claims` is the table of waiting notes that are lent to agents. When an agent asks for notes to explain, the server lists them and marks them as held by that agent, so other agents get different notes.

## Source

- `crates/cw-server/src/claims.rs` — definition of `Claims`

## How it works

`list_explanations` in [[Api Module]] calls `expire` to drop old claims, `others` to find notes held by other agents, and `claim` to lend the listed notes to the asker. `write_explanation` calls `release` for the note just written, and `renew` to keep that agent's other claims alive. A claim lasts one period, set in [[Claims Module]]. `claim` and `expire` return true when something changed, so the caller can raise the notes counter that clients watch.

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

## Gotchas

- Claim and list under one lock, or two agents can get the same note.
- Asking again with the same agent id keeps its own notes in the list. `others` leaves out only other agents' notes.
- With `None` as agent, `others` returns every held note.

## See also

- [[_index]]
- [[Claims Module]]
