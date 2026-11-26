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

- [[Explanations Module]] — from `Outcome`, `Queue`, `count`, `fits_all`
- [[cw-wiki]] — from `a_submission_is_woven_into_the_note_and_recorded`, `every_lists_all_waiting_notes_and_marks_the_claimed`, `the_queue_lists_what_waits_and_counts_all_of_it`, `the_waiting_line_orders_a_batch_for_each_host`
- [[Briefing Module]] — from `session_start`

## See also

- [[_index]]
- [[Explanations Module]]
