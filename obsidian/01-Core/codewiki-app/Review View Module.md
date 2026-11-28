---
tags: [core, codewiki-app, module]
---

# Review View Module

> The review page as the app's view draws and runs it.

## What it does

A proposal is a change to a note that the owner edited by hand. A sync makes one when the code under the note changed, and a coding agent makes one when it explains such a note. The text waits until the owner approves or rejects it. This module is the review page of the app: it draws the queue of waiting proposals and runs the decisions.

The queue is at the left, each row with a checkbox, the note, its kind and why it was made. The open proposal is at the right, drawn by the cached page view (`doc_element`), with the Approve and Reject buttons above it. A click on a row shows its proposal in place, with no history entry. After a decision the queue loads again quietly and the next proposal opens.

The status poll counts what waits in every repository; a count that rises is news (`take_proposal_counts`). What is selected, what opens next and what the user is told is worked out in `review.rs`.

## Source

- `crates/codewiki-app/src/review_view.rs` — module `codewiki_app::review_view`

## How it works

Showing: `show_review` selects the first proposal alone and calls `show_proposal`. That sends a quiet GET for the proposal (no progress bar) and remembers it as wanted. When the answer arrives, it is dropped if another proposal was asked for since, or the repository changed. With `Show::InPlace` it is also dropped if the user left the page. Otherwise the page becomes the proposal's page.

Clicking: `click_review_row` updates the selection with plain, control or shift click rules and opens the row in place when a click chose a new proposal. The checkboxes toggle one row or all rows. When a sync proposal and an agent proposal for one note are both selected, `review_clash` shows a warning.

Retire proposals: `review_retire` lists the notes the open retire proposal would delete, each with a check box. A ticked note is deleted; an unticked one is kept as history (`review.keep`). Approve counts the notes (`retire_label`); Reject reads "Keep all as history".

Deciding: `decide_review` takes the ids from `review_targets` (the selection when it holds anything, else the open proposal) and runs `decide` off the UI thread. `decide` posts to the proposal's own approve or reject route for one id, and calls `Backend::decide_proposals` for several. A retire proposal always goes by its own route, which alone takes the notes to keep. When results return: notices are shown, pages of written notes are forgotten in every tab, the notes reload, and `after_decision` picks the next proposal or shows the empty review page.

Counts: `take_proposal_counts` compares the new status counts with the old, tells the user about rises, and reloads the open repository's queue quietly. `review_queue_grew` opens the first proposal when the empty review page was shown and proposals arrive. `review_details` fills the details pane.

## Items

- `pub const QUEUE_WIDTH: f32` — width of the queue beside the open proposal.
- `const QUEUE_HEAD: f32` — the row above the queue, with the checkbox that selects every row: `ListSubheader.js`, `lineHeight: '48px'`.
- `const RETIRE_LIST_HEIGHT: f32` — how tall the list of a retire proposal's notes grows before it scrolls: the text of the proposal under it must stay in sight.
- `pub(crate) enum Show` — where the review page shows a proposal.
- `fn empty_review() -> Page` — the review page when nothing waits.
- `decide` — decide the proposals `ids` of the repository `repo`. One goes by its own route, which every backend has, and a refusal is its result; several go in one call, which an older backend refuses with an error that says so. Blocks; run it off the UI thread. `keep` names a retire proposal among them and the notes of it to keep: only its own route takes those, so it goes there by itself and the others as before; the results come back in the order of `ids`.

## Depends on

- [[Backend]] — uses `Backend`, `decide_proposals`
- [[codewiki-app]] — uses `Page`
- [[cw-client]] — uses `DecideResult`
- [[Live Module]] — uses `review_empty_page`

## Used by

- [[codewiki-app]] — from `decide_review`, `review_queue`, `review_queue_width`, `review_retire`

## Gotchas

- The single-id route exists on every backend. The many-id call is refused by an older backend with an error that says so, so one proposal never depends on the newer call.
- `decide` blocks. Never call it on the UI thread.
- The Approve and Reject buttons wrap onto two lines in a narrow window. Without that, Reject ran under the details pane where no click reached it (found at 1024x768 in the clean-Windows test).
- A proposal that fails to load sets an error, and the error is cleared again once a later proposal loads.
- With nothing waiting, the queue is not drawn at all and its width is zero.

## See also

- [[_index]]
- [[codewiki-app]]
