---
tags: [core, codewiki-app, module]
---

# Diagrams Module

> Drawn Mermaid diagrams, kept so that a note opens without waiting.

## Source

- `crates/codewiki-app/src/diagrams.rs` — module `codewiki_app::diagrams`

## How it works

Main items:

- `Diagrams` — struct
- `Key` — A diagram is known by the palette it is drawn in and its Mermaid source.
- `Lookup` — what the store knows about one diagram.
- `LIMIT` — drawn diagrams kept before the store starts over.

## Depends on

- [[Palette]] — uses `Palette`

## Used by

- [[codewiki-app]] — from `AppView`, `draw_queued_diagrams`, `prepare_page`
- [[Instance Module]] — from `release`
- [[Win32 Module]] — from `keep_open`

## See also

- [[_index]]
- [[codewiki-app]]
