---
tags: [core, codewiki-app, module]
---

# Suggest Module

> What the search field suggests while it is typed in: the hits of its text, looked up at each pause in typing, in a list under the field.

## What it does

The search box in the app's toolbar shows a list of hits under itself while you type, so you see results without pressing Enter. This module keeps the state of that list: the hits it shows, whether it is open, which row the arrow keys point at, and the lookup that waits for you to stop typing. The app's main view in [[codewiki-app]] owns one Suggestions value and changes it from the search box's events. [[Autocomplete Module]] draws the list.

The search box keeps the keyboard the whole time. Down moves from the box into the list and on down it; Up moves back up and, from the first row, back to the box. Enter opens the highlighted hit. With no row highlighted, Enter opens the full Search page, as it did before the list existed.

## Source

- `crates/codewiki-app/src/suggest.rs` — module `codewiki_app::suggest`

## How it works

The picture shows the path from a key to the list. `AppView::suggest` in [[codewiki-app]] starts it on every change of the text. An empty text calls `clear` instead, and the list goes away.

The lookup asks `GET /v1/repos/{id}/search` with `mode=text` and `limit` set to `LIMIT`. Text mode matches the start of each word and never calls the embedder, so asking at every pause stays cheap. `show` hands the hits to `hit_groups` in [[Live Module]], which sorts them into Notes, Symbols and Files.

The root view catches Up, Down and Escape while the list is open, before the search box can use them, and calls `up`, `down` or `close`. Enter calls `chosen`: a highlighted hit is opened with the action that `hit_target` gave it; otherwise the full search runs.

`close` hides the list but keeps its hits, and drops a lookup that has not answered. Escape, Enter, a chosen row and the box losing the focus all close it. When the box gets the focus back, or is clicked, `reopen` shows the list again, but only if the box still holds the text the hits were for.

## Items

- `pub const PAUSE: Duration` — how long typing must pause before the field's text is looked up.
- `pub const LIMIT: usize` — how many hits the list shows.
- `pub struct Suggestions` — public struct; used by `AppView`.

## Depends on

- [[Live Module]] — uses `HitTarget`, `hit_groups`
- [[Menu Module]] — uses `AnchorBounds`

## Used by

- [[codewiki-app]] — from `AppView`, `choose_suggestion`, `new`, `render`

## Gotchas

- Rows are counted across the groups, in the order Notes, Symbols, Files. The drawn list and `target` must count the same way, or Enter opens a different hit from the highlighted one.
- After the text changes, the old hits stay in view until new ones come (`typed`), but no row is highlighted. Enter in that moment runs the full search of the new text.
- The lookup does not go through `AppView::run`. It shows no busy line and no error banner: a failed lookup only closes the list.
- A backend older than this module does not know `mode=text` and ignores it. It answers with the full search, which asks the embedder each time.

## See also

- [[_index]]
- [[codewiki-app]]
