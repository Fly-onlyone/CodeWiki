---
tags: [core, codewiki-app, module]
---

# Diagrams Module

> Drawn Mermaid diagrams, kept so that a note opens without waiting.

## What it does

Drawing one diagram takes seconds in a debug build, and the first of a session also reads every installed font. The view therefore never draws: it asks here, shows a placeholder for what is missing, and has the missing diagrams drawn off the UI thread. No view types, so it is unit-tested.

## Source

- `crates/codewiki-app/src/diagrams.rs` — module `codewiki_app::diagrams`

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

## See also

- [[_index]]
- [[codewiki-app]]
