---
tags: [core, cw-wiki]
---

# Counts

> How many notes wait for an agent.

## Source

- `crates/cw-wiki/src/explanations.rs` — definition of `Counts`

## Items

Declared as `pub struct Counts`.

Members:

- `pub fn total(self) -> usize` — public method, returns `usize`.

## Used by

- [[Explanations Module]] — from `Outcome`, `Queue`, `count`, `waiting_counts`
- [[cw-wiki]] — from `a_submission_is_woven_into_the_note_and_recorded`, `the_queue_lists_what_waits_and_counts_all_of_it`
- [[Briefing Module]] — from `session_start`

## See also

- [[_index]]
- [[Explanations Module]]
