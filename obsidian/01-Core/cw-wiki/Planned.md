---
tags: [core, cw-wiki]
---

# Planned

> `Planned` struct defined in `cw_wiki::plan`.

## Source

- `crates/cw-wiki/src/plan.rs` — definition of `Planned`

## How it works

Declared as `pub struct Planned`.

## Depends on

- [[Plan Module]] — uses `Subject`
- [[NoteKind]] — uses `NoteKind`

## Used by

- [[RenderCtx]] — from `illustration`, `render`, `render_crate`, `render_crate_flow`
- [[Generate Module]] — from `generate`, `planned_path`
- [[Plan Module]] — from `Plan`, `plan`
- [[LlmWriter]] — from `illustrate`, `rewrite`
- [[ProseWriter]] — from `illustrate`, `rewrite`
- [[Writer Module]] — from `read_sources`
- [[Render Module]] — from `new`

## See also

- [[_index]]
- [[Plan Module]]
