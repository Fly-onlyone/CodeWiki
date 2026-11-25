---
tags: [core, cw-server, module]
---

# Pending Module

> How many proposals wait in each repository, for `GET /v1/status`.

## What it does

Counting reads every proposal of every registered repository, and one on a network share or on a disk that sleeps can take seconds. The app asks for the status every 5 s and takes an answer slower than 3 s for a lost backend, so the status waits for a count at most `WAIT`. A count that takes longer goes on in the background. One count runs at a time: a question asked meanwhile joins the count that runs.

An answer gives only the repositories counted while it waited. A count made before the question may be older than what the app has loaded since, and the app would take the difference for news; a repository left out of an answer is only compared at the next one.

## Source

- `crates/cw-server/src/pending.rs` — module `cw_server::pending`

## How it works

What the functions of this module call:

1. `ProposalCounts::get` calls `ProposalCounts::start`, then `fresh`, then `lock`. `ProposalCounts::start` calls `lock`.
2. `End::drop` calls `lock`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

## See also

- [[_index]]
- [[cw-server]]
