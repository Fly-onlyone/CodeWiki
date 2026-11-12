---
tags: [core, cw-wiki, module]
---

# Plan Module

> Concept selection: decide which notes a repository gets.

## Source

- `crates/cw-wiki/src/plan.rs` — module `cw_wiki::plan`

## How it works

Main items:

- [[Repo]] — lookup tables over the scanned facts.
- [[Planned]] — struct
- `FlowMessage` — one step of a startup flow: `from` note calls into `to` note.
- `Subject` — enum
- `ZONE_CORE` — const
- `title_case` — `hash` -> `Hash`, `note_lifecycle` -> `Note Lifecycle`.
- `Plan` — struct
- `ZONE_ARCHITECTURE` — const

## Depends on

- [[Repo]] — uses `Repo`, `children_of`, `descendants`, `is_test`
- [[Unit]] — uses `Unit`, `is_rust`
- [[Units]] — uses `all_rust`, `index_of`
- [[Model Module]] — uses `Edge`
- [[Language]] — uses `has_symbol_support`
- [[Symbol]] — uses `Symbol`
- [[SymbolKind]] — uses `is_type`
- [[Planned]] — uses `Planned`

## Used by

- [[RenderCtx]] — from `RenderCtx`, `new`, `render_home`, `render_startup_flow`
- [[Generate Module]] — from `generate`
- [[Planned]] — from `Planned`
- [[Repo]] — from `is_test`

## See also

- [[_index]]
- [[cw-wiki]]
