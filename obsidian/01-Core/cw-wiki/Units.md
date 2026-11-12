---
tags: [core, cw-wiki]
---

# Units

> Every unit of a repository and the lookup from code to unit.

## Source

- `crates/cw-wiki/src/units.rs` — definition of `Units`

## How it works

Declared as `pub struct Units`.

Public API:

- `pub fn rust_index(&self, qualified: &str) -> Option<usize>` — index of the crate a Rust path starts in.
- `pub fn index_of(&self, symbol: &Symbol) -> Option<usize>` — index of the unit that holds `symbol`.
- `pub fn all_rust(&self) -> bool`
- `pub fn by_name(&self, name: &str) -> Option<&Unit>`

## Depends on

- [[Unit]] — uses `Unit`, `is_rust`
- [[Language]] — uses `from_path`
- [[Symbol]] — uses `Symbol`

## Used by

- [[RenderCtx]] — from `illustration`, `render_crate`, `render_crate_flow`, `render_home`
- [[Repo]] — from `Repo`, `build`, `top_modules`, `unit_index`
- [[Plan Module]] — from `plan`
- [[Units Module]] — from `discover`

## See also

- [[_index]]
- [[Units Module]]
