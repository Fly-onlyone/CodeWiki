---
tags: [core, cw-illustrate]
---

# cw-illustrate

> CodeWiki's illustration component library and structural illustration builders.

## Source

- `crates/cw-illustrate/Cargo.toml` — package manifest
- `crates/cw-illustrate/src/lib.rs` — crate root

## How it works

Builds a library.

Defined at the crate root:

- `quote` — quote text as an OpenUI Lang string literal.
- `EXTENSION` — file extension of saved illustrations.
- `check` — parse and check an illustration against the library.
- `library` — the components an illustration may use.
- [[Figure]] — one headline figure.
- `overview` — an overview card: a title, a one-line summary, headline figures and an optional bar chart.
- `steps` — A numbered sequence of steps, for flows.
- `number` — function

## Depends on

- [[Eval Module]] — uses `Document`, `evaluate`
- [[Library]] — uses `Library`, `new`
- [[Parser Module]] — uses `parse`
- [[Figure]] — uses `Figure`

## Used by

- [[Generate Module]] — from `generate`, `illustration_path`
- [[codewiki-app]] — from `open_note`
- [[cw-wiki]] — from `crate_and_flow_notes_get_valid_illustrations`
- [[RenderCtx]] — from `illustration`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
