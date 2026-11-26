---
tags: [core, codewiki-app, module]
---

# Diagrams Module

> Drawn Mermaid diagrams, kept so that a note opens without waiting.

## What it does

Drawing one diagram takes seconds in a debug build, and the first of a session also reads every installed font. The view therefore never draws: it asks here, shows a placeholder for what is missing, and has the missing diagrams drawn off the UI thread. No view types, so it is unit-tested.

## Source

- `crates/codewiki-app/src/diagrams.rs` — module `codewiki_app::diagrams`

## How it works

`Diagrams` keeps three collections: finished drawings (`done`), requests not yet handed out (`queued`) and jobs handed out but not yet stored (`running`).

1. The view calls `lookup(source, palette)` while it lays out a page. A finished diagram returns `Ready`, with an image or with `None` when the source cannot be drawn.
2. A missing one returns `Pending`. It is queued once. A second `lookup` while it is queued or running adds nothing.
3. The app calls `take_queued`, which moves every queued key to `running` and returns them. It draws them off the UI thread through [[cw-mermaid]].
4. Each result goes back with `store`, which clears the job and saves the image, or `None` for a failure.
5. The next `lookup` finds it ready.

The key holds the palette as well as the source, so a dark and a light drawing of one diagram are separate entries.

## Items

- `pub type Key = (Palette, String)` — A diagram is known by the palette it is drawn in and its Mermaid source.
- `const LIMIT: usize` — drawn diagrams kept before the store starts over. Notes hold one diagram each, so this covers every note a session is likely to visit.
- `pub enum Lookup` — what the store knows about one diagram.
- `pub struct Diagrams` — public struct; used by `AppView`.

## Depends on

- [[Palette]] — uses `Palette`

## Used by

- [[codewiki-app]] — from `AppView`, `DiagramViewer`, `draw_queued_diagrams`, `prepare_page`
- [[Instance Module]] — from `release`
- [[Win32 Module]] — from `keep_open`

## Gotchas

- A failed drawing is stored as `None`, so a broken diagram is not tried again on every frame.
- Every `take_queued` key must get a `store`. Otherwise the key stays in `running` and is never queued again.
- The store holds at most 32 drawings (`LIMIT`). It empties completely, not one at a time, and only when a new diagram is asked for with the store full. A page whose diagrams are all drawn never triggers it. A test checks that a page with more diagrams than the limit still settles within a few rounds.
- The module has no view types, so its tests need no window.

## See also

- [[_index]]
- [[codewiki-app]]
