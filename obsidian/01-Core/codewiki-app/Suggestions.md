---
tags: [core, codewiki-app]
---

# Suggestions

> `Suggestions` struct defined in `codewiki_app::suggest` with 14 methods.

## What it does

`Suggestions` is the state of one suggestion list under a search field: the hits in groups, whether the list is open, and which row the arrow keys point at. The app keeps two, one for the toolbar's search and one for Home's search of the projects opened before. It holds no drawing code and does no lookup itself. The app fills it with `show` (backend hits) or `show_targets` (hits the app found in its own memory), and the [[Suggest Module]] explains the flow around it.

## Source

- `crates/codewiki-app/src/suggest.rs` — definition of `Suggestions`

## How it works

Hits live in groups, each with a heading. A row is counted across all groups in order, so `target(index)` and the highlighted row agree with what is drawn.

`show` and `show_targets` store the trimmed query, replace the groups, open the list and clear the highlight. `show` first sorts raw JSON hits with `hit_groups` from the [[Live Module]].

`highlighted` is `None` while the keys act on the field. `down` moves into the list and stops at the last row. It does nothing for an empty list. `up` moves back, and from the first row returns to the field. `typed` clears the highlight when the text changes but leaves the old hits in view. `chosen` returns the highlighted hit, which Enter opens.

`close` hides the list but keeps hits and query, and drops the pending lookup in `task`. `clear` also forgets the hits. `reopen` shows the list again only if the field's trimmed text equals the stored query.

## Items

Declared as `pub struct Suggestions`.

Members:

- `pub fn show(&mut self, query: &str, hits: &Value)` — show the hits of `query`, with no row highlighted.
- `pub fn show_targets(&mut self, query: &str, groups: Vec<(&'static str, Vec<HitTarget>)>)` — show `groups`, found for `query` in the app itself, with no row highlighted: Home's search of the projects opened before.
- `pub fn close(&mut self)` — hide the list, keeping its hits for when the field has the focus again, and drop a lookup that has not answered.
- `pub fn clear(&mut self)` — forget the hits: the field is empty.
- `pub fn typed(&mut self)` — the text changed: the keys act on the field again until new hits come. The old ones stay in view meanwhile.
- `pub fn reopen(&mut self, text: &str) -> bool` — show the list again if its hits are those of `text`. Whether it is open now.
- `pub fn is_open(&self) -> bool` — public method, returns `bool`.
- `pub fn groups(&self) -> &[(&'static str, Vec<HitTarget>)]` — public method, returns `&[(&'static str, Vec<HitTarget>)]`; called by `Suggestions::show_targets`.
- `pub fn highlighted(&self) -> Option<usize>` — public method, returns `Option<usize>`.
- `fn count(&self) -> usize` — private method, returns `usize`; called by `Suggestions::down`.
- `pub fn target(&self, index: usize) -> Option<&HitTarget>` — the hit at `index`, counted across the groups.
- `pub fn chosen(&self) -> Option<&HitTarget>` — the highlighted hit, which Enter opens.
- `pub fn down(&mut self)` — down: into the list from the field, then on down it, stopping at the last row.
- `pub fn up(&mut self)` — up: back up the list, and from its first row back to the field.

## Depends on

- [[Live Module]] — uses `HitTarget`, `hit_groups`
- [[Menu Module]] — uses `AnchorBounds`

## Used by

- [[codewiki-app]] — from `AppView`, `choose_recent`, `choose_suggestion`, `home_search_key`

## Gotchas

- `task` is public: dropping a lookup cancels it, so a new lookup must replace the old one rather than sit beside it.
- `reopen` compares the trimmed text. After `clear` the query is empty, so it never reopens.
- An empty result list is still open. `down` then does nothing, so Enter falls back to the field's own action.
- `anchor` is where the field was drawn last frame. The list hangs under it, so it is wrong for one frame after a move.

## See also

- [[_index]]
- [[Suggest Module]]
