---
tags: [core, cw-server, module]
---

# Pending Module

> How many proposals wait in each repository, for `GET /v1/status`.

## What it does

The app polls `GET /v1/status` every 5 seconds and treats an answer slower than 3 seconds as a lost backend. Counting proposals reads every registered repository, and a network share or a sleeping disk can take seconds. This module counts in the background and lets the status wait only `WAIT` (one second) for the result. A slow count keeps going. One count runs at a time, and a new question joins it.

## Source

- `crates/cw-server/src/pending.rs` — module `cw_server::pending`

## How it works

`ProposalCounts::get` calls `start`. If a count runs, `start` returns it and a watch channel. If none runs, it creates a `Count`, stores it in the slot and runs the counting closure on a blocking thread. That closure reports each repository through a callback, which saves the number with the time it was found. `get` waits for the count to end or for `wait` to pass, then calls `fresh`.

`End` is held by the counting thread. Its `Drop` clears the slot and signals the end, also after a panic, so the next question starts a new count.

## Items

- `pub(crate) type Counts = BTreeMap<String, usize>` — pending proposals by repository id.
- `pub(crate) type Found = Option<usize>` — what a count found for one repository: how many proposals wait there, or `None` when it is left out (its folder is gone, its vault cannot be read).
- `pub(crate) const WAIT: Duration` — how long the status waits for a count.
- `pub(crate) struct ProposalCounts` — the count that runs, if one does.
- `struct Count` — A count under way.
- `fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T>` — private function, returns `MutexGuard<'_, T>`; called by `End::drop`, `ProposalCounts::get`, `ProposalCounts::start`.
- `struct End` — ends a count when it is dropped, a panic included, and lets the next one run.
- `fn fresh(found: &BTreeMap<String, (Found, Instant)>, asked: Instant) -> Counts` — what a count found at or after `asked`, less the repositories it left out.

## Used by

- [[Api Module]] — from `backend_status`, `pending_proposals`
- [[Inner]] — from `Inner`

## Gotchas

- An answer holds only repositories counted after the question was asked. An older count could be older than what the app loaded since, and the app would show the difference as news. A repository missing from an answer is simply compared at the next one.
- A repository whose folder is gone or whose vault cannot be read gives `None` and is left out.
- A question that joins a running count starts no count of its own, so its closure never runs.
- What a count found before a panic is still shown.

## See also

- [[_index]]
- [[cw-server]]
