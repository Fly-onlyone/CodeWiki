---
tags: [core, codewiki-app, module]
---

# Review Module

> The review queue as the app keeps it: the pending proposals, which of them are selected, which opens after a decision, and what the user is told.

## What it does

No UI here; `review_view.rs` draws the page and runs the requests.

A sync proposes a change to a note someone edited by hand, and a coding agent proposes its explanation of such a note. Each waits for a decision, one at a time or several at once.

## Source

- `crates/codewiki-app/src/review.rs` — module `codewiki_app::review`

## How it works

1. `AppView::take_notes` in [[codewiki-app]] calls `pending`.
2. `AppView::review_queue` in [[codewiki-app]] calls `Click::with_keys`.
3. `AppView::decide_review` in [[codewiki-app]] calls `after_decision`. `after_decision` calls `next_after`.
4. `AppView::review_clash` in [[codewiki-app]] calls `clash_warning`.
5. `AppView::review_details` in [[codewiki-app]] calls `counts`.

11 more entry points: `decided_notices`, `rise_key`, `rise_notice`, `targets`, `written`, `forget_written`, and 5 more.

## Items

- `pub enum Kind` — what made a proposal.
- `pub struct Item` — one pending proposal, as the queue lists it.
- `pub fn pending(proposals: &Value) -> Vec<Item>` — the pending proposals in what `GET /v1/repos/{id}/proposals` answered, in its order. A proposal with no kind is a sync's, as on the backend.
- `pub struct Selection` — which proposals of the queue are selected, by id.
- `pub fn next_after(order: &[String], decided: &[String], current: Option<&str>) -> Option<String>` — the proposal to open after a decision on `decided`: the first one after `current` in `order` that was not decided, else the nearest one before it, else none. With no `current`, or one not in `order`, the first that was not decided.
- `after_decision` — the proposal to show once the queue has been loaded again after a decision. `order` is the queue before, `pending` after. The open one stays when it was not decided and still waits; otherwise the next one (`next_after`), and the first of the queue when that finds none. `None` only when nothing waits.
- `pub fn clash_warning(items: &[Item], selection: &Selection) -> Option<String>` — the warning shown while the selection holds a sync's and an agent's proposal for the same note: approving both writes one, and the other no longer fits the note. `None` when there is no such pair.
- `pub fn counts(items: &[Item], selected: usize) -> Vec<String>` — the lines of the details pane beside the queue.
- `pub fn decided_notices(approve: bool, results: &[DecideResult], items: &[Item]) -> Vec<Notice>` — what the user is told after a decision: what went through, and a warning that lists what did not. `items` is the queue before the decision, for the names of the notes.
- `pub fn rise_key(id: &str) -> String` — the key of the notice that new proposals wait in the repository `id`.
- `pub fn rise_notice(id: &str, name: &str, increase: usize) -> Notice` — the notice that new proposals wait in the repository `id`, named `name`.
- `pub enum Targets` — what the buttons above the open proposal decide.
- `targets` — what a decision acts on: the selection whenever it holds anything, the open proposal only while nothing is selected. A selection of the open proposal alone is that proposal, and so is a selection of the one whose detail is `loading`, which a plain click on its row is about to open. Only what waits in `order` counts.
- `pub fn written(approve: bool, results: &[DecideResult], items: &[Item]) -> Vec<String>` — the notes an approval wrote, by vault path: none for a rejection.
- `forget_written` — drop from `history` every page of a note in `written`, which shows the note as it was before: going back to one and editing it would save the old text over the approved one. `note` gives the path of a page that is a note, `None` for any other page, which stays.
- `to_reload` — which tabs must load their note again when next shown: each tab, given its page (a note's path, or `None` for any other page), that is not the `active` one and shows a note in `written`. Such a tab holds the note as it was, and an edit there would save that over the approved text. The active tab's slot is an old copy; the page on screen is dealt with apart.
- `pub fn own_work(marked: bool, busy: Option<&str>) -> bool` — whether the app itself is busy with work that may add proposals to the open repository: its own sync, and the loading of the open repository's notes. `marked` says that the work running now is such work; a report that loads is not. Counts that rise meanwhile are the app's own doing; once the queue has loaded, it is what later counts are measured from (`loaded`).
- `news` — the repositories to tell the user about: those whose count rose between `before` and `after` (`crate::status::rises`), less the open one while the app's own work runs, since the app knows of those proposals itself. Each is its id, its new count and by how much it rose.
- `pub fn loaded(counts: &mut Option<HashMap<String, usize>>, open: &str, pending: usize)` — take the queue the app loaded for the open repository `open`, `pending` proposals long, as the count its rises are measured from: what the app loaded itself is no news, however fast its sync ran. Nothing changes before the first answer that counts.
- `pub fn take_counts(known: &mut Option<HashMap<String, usize>>, answer: HashMap<String, usize>)` — take an answer of `GET /v1/status` into the counts the app knows: each repository in it gets its new count, and one it leaves out keeps the count known before. On slow storage the backend leaves out what it did not count in time, rather than give a count that may be older than what the app loaded since (`loaded`).

And 6 more items.

## Depends on

- [[cw-client]] — uses `DecideResult`
- [[Checkbox Module]] — uses `Check`
- [[Status Module]] — uses `rises`
- [[Toasts Module]] — uses `Notice`

## Used by

- [[codewiki-app]] — from `AppView`, `click_review_row`, `decide_review`, `forget_written_notes`
- [[Toasts]] — from `offer`, `push`, `sent`
- [[Instance Module]] — from `listen`
- [[Base Module]] — from `new`
- [[Startup Module]] — from `touched`

## See also

- [[_index]]
- [[codewiki-app]]
