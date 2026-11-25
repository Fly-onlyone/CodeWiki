---
tags: [core, codewiki-app, module]
---

# Review View Module

> The review page as the app's view draws and runs it.

## What it does

The queue of pending proposals is at the left, the open proposal at the right, drawn by the cached page view (`doc_element`), and above it the buttons that decide. A click on a row shows its proposal in place, with no history entry. One proposal is decided by its own routes, which every backend has; several go to the backend in one call. After a decision the queue loads again quietly and the next proposal opens.

The status poll counts what waits in every repository; a count that rises is news (`take_proposal_counts`). What is selected, what opens next and what the user is told is worked out in `review.rs`.

## Source

- `crates/codewiki-app/src/review_view.rs` — module `codewiki_app::review_view`

## How it works

1. `AppView::decide_review` and `AppView::show_review` in [[codewiki-app]] call `empty_review`. `empty_review` calls `review_empty_page` in [[Live Module]].
2. `AppView::decide_review` in [[codewiki-app]] calls `decide`.

## Items

- `pub const QUEUE_WIDTH: f32` — width of the queue beside the open proposal.
- `const QUEUE_HEAD: f32` — the row above the queue, with the checkbox that selects every row: `ListSubheader.js`, `lineHeight: '48px'`.
- `pub(crate) enum Show` — where the review page shows a proposal.
- `fn empty_review() -> Page` — the review page when nothing waits.
- `decide` — decide the proposals `ids` of the repository `repo`. One goes by its own route, which every backend has, and a refusal is its result; several go in one call, which an older backend refuses with an error that says so. Blocks; run it off the UI thread.

## Depends on

- [[Backend]] — uses `Backend`, `decide_proposals`
- [[codewiki-app]] — uses `Page`
- [[cw-client]] — uses `DecideResult`
- [[Live Module]] — uses `review_empty_page`

## Used by

- [[codewiki-app]] — from `decide_review`, `review_queue`, `review_queue_width`, `show_proposal`

## See also

- [[_index]]
- [[codewiki-app]]
