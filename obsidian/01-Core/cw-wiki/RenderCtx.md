---
tags: [core, cw-wiki]
---

# RenderCtx

> `RenderCtx` struct defined in `cw_wiki::render` with 22 methods.

## Source

- `crates/cw-wiki/src/render.rs` — definition of `RenderCtx`

## How it works

Declared as `pub struct RenderCtx<'a>`.

Public API:

- `pub fn link(&self, id: &str) -> Option<String>` — wikilink to the note with `id`, or `None` when it is not in the vault.
- `pub fn illustration(&self, note: &Planned) -> Option<String>` — structural illustration for a note, as OpenUI Lang source.
- `pub fn new( repo: &'a Repo<'a>, plan: &'a Plan, registry: &'a Registry, repo_name: &'a str, ) -> Self`
- `pub fn render(&self, note: &Planned) -> String`

## Depends on

- [[Render Module]] — uses `Doc`, `MAX_LISTED`, `MAX_NAMES`, `MAX_UNIT_MODULES`
- [[Plan Module]] — uses `FlowMessage`, `Plan`, `ZONE_ARCHITECTURE`, `ZONE_CORE`
- [[Repo]] — uses `Repo`, `children_of`, `module_count`, `rank_of`
- [[Unit]] — uses `Unit`, `declares`, `is_rust`
- [[Units]] — uses `all_rust`, `by_name`, `index_of`
- [[cw-illustrate]] — uses `overview`, `steps`
- [[Language]] — uses `Language`, `from_path`
- [[Registry]] — uses `Registry`, `by_id`

## Used by

- [[Generate Module]] — from `generate`

## See also

- [[_index]]
- [[Render Module]]
