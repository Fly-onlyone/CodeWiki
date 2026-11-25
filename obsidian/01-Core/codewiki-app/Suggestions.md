---
tags: [core, codewiki-app]
---

# Suggestions

> `Suggestions` struct defined in `codewiki_app::suggest` with 13 methods.

## What it does

`Suggestions` is the state of the list that hangs under the app's search box while you type (see [[Suggest Module]]). It holds the trimmed text the hits are for, the hits grouped by kind, whether the list is open, the highlighted row, the lookup task that is still waiting, and where the box is drawn on screen. The app's main view in [[codewiki-app]] owns one and changes it from the search box's events; [[Autocomplete Module]] draws what it holds.

## Source

- `crates/codewiki-app/src/suggest.rs` — definition of `Suggestions`

## How it works

The list moves between three states: closed with no hits, open, and closed with its hits kept.

- `show` stores the hits of a text, grouped by `hit_groups` from [[Live Module]], and opens the list with no row highlighted.
- `typed` runs on each change of the text. It only removes the highlight; the old hits stay until `show` brings new ones.
- `close` hides the list, removes the highlight and drops `task`, which cancels a lookup still waiting. The hits stay.
- `clear` closes the list and also forgets the text and the hits. It runs when the search box is empty.
- `reopen` opens the list again only when its stored text equals the box's text, trimmed. It returns whether the list is open now.

The highlight is `None` while the keys act on the box. `down` goes from `None` to the first row and then down, stopping at the last; `up` goes back up and from the first row back to `None`. `target` finds the hit at a row number counted across all groups, and `chosen` is the hit at the highlight, which Enter opens.

## Items

Declared as `pub struct Suggestions`.

Members:

- `pub fn show(&mut self, query: &str, hits: &Value)` — show the hits of `query`, with no row highlighted.
- `pub fn close(&mut self)` — hide the list, keeping its hits for when the field has the focus again, and drop a lookup that has not answered.
- `pub fn clear(&mut self)` — forget the hits: the field is empty.
- `pub fn typed(&mut self)` — the text changed: the keys act on the field again until new hits come. The old ones stay in view meanwhile.
- `pub fn reopen(&mut self, text: &str) -> bool` — show the list again if its hits are those of `text`. Whether it is open now.
- `pub fn is_open(&self) -> bool` — public method, returns `bool`.
- `pub fn groups(&self) -> &[(&'static str, Vec<HitTarget>)]` — public method, returns `&[(&'static str, Vec<HitTarget>)]`.
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

- [[codewiki-app]] — from `AppView`, `choose_suggestion`, `new`, `render`

## Gotchas

- `task` must hold the lookup's task for as long as it should run: dropping or replacing it cancels the lookup.
- `anchor` is filled in when the search box is drawn. Until the box has been drawn once, there is no place to hang the list, and it is not shown.
- `down` does nothing when there are no hits, so the highlight stays `None` and Enter runs the full search.

## See also

- [[_index]]
- [[Suggest Module]]
