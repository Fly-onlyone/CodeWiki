---
tags: [core, cw-wiki]
---

# Counts

> How many notes wait for an agent.

## What it does

A pair of numbers that says how many notes wait for a coding agent to explain them. One number counts notes whose explanation may be out of date. The other counts notes no agent has explained yet. The session briefing and the queue use it to tell the agent how much work waits.

## Source

- `crates/cw-wiki/src/explanations.rs` — definition of `Counts`

## How it works

`Counts` has two fields. `behind` counts notes that were explained, but a file the explanation was written from has changed since. `unexplained` counts notes written by rules only. `total` adds them.

`count` builds it from the list of waiting notes: it counts those marked `Behind`, and everything else is `unexplained`. `waiting_counts` does this for a whole repository's vault. `queue` also carries a `Counts` in its `waiting` field, counted over the whole vault, not only the notes that fit the filter.

## Items

Declared as `pub struct Counts`.

Members:

- `pub fn total(self) -> usize` — public method, returns `usize`.

## Used by

- [[Explanations Module]] — from `Outcome`, `Queue`, `count`, `fits_all`
- [[cw-wiki]] — from `a_submission_is_woven_into_the_note_and_recorded`, `every_lists_all_waiting_notes_and_marks_the_claimed`, `the_queue_lists_what_waits_and_counts_all_of_it`, `the_waiting_line_orders_a_batch_for_each_host`
- [[Briefing Module]] — from `session_start`

## Gotchas

- A note only counts as waiting when nobody edited it. `untouched` checks that its origin is generated and that its body hash still matches. A note its owner edited is not counted, because what an agent writes for it would only be a proposal, again on every run.
- Notes whose kind cannot be explained are not counted.
- The count covers the whole vault, even when the queue was narrowed by a folder or a file list. Notes held by another agent are still counted here; they are only left out of the listed notes.

## See also

- [[_index]]
- [[Explanations Module]]
