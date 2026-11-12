---
tags: [core, cw-wiki, module]
---

# Render Module

> Rendering of planned notes into Markdown in the vault's house style.

## Source

- `crates/cw-wiki/src/render.rs` — module `cw_wiki::render`

## How it works

Main items:

- [[RenderCtx]] — struct
- `clean_inline` — make doc-comment text safe to place in a note: rustdoc links become code, Markdown links keep only their text, and wikilink brackets are defused.
- `first_sentence` — first sentence of a doc comment, ending with a period.
- `Doc` — accumulates a note's sections; empty ones are dropped.
- `zone_label` — `01-Core` -> `Core`; other folder names are kept.
- `PROPER_NOUNS` — product and technology names that keep their capital at the start of a list item.
- `ensure_period` — function
- `front` — function

## Depends on

- [[Repo]] — uses `Repo`, `children_of`, `subtree_rank`
- [[Language]] — uses `Language`, `from_path`
- [[Symbol]] — uses `Symbol`
- [[Planned]] — uses `Planned`

## Used by

- [[RenderCtx]] — from `RenderCtx`, `composition`, `dep_bullets`, `illustration`
- [[Decisions Module]] — from `status_of`, `why`
- [[Generate Module]] — from `cited_sources`

## See also

- [[_index]]
- [[cw-wiki]]
