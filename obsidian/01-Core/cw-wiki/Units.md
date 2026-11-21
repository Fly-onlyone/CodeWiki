---
tags: [core, cw-wiki]
---

# Units

> Every unit of a repository and the lookup from code to unit.

## Source

- `crates/cw-wiki/src/units.rs` — definition of `Units`

## How it works

What the functions of this type call:

1. `index_of` calls `Language::from_path` in [[Language]], then `rust_index`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct Units`.

Members:

- `pub fn index_of(&self, symbol: &Symbol) -> Option<usize>` — index of the unit that holds `symbol`.
- `pub fn rust_index(&self, qualified: &str) -> Option<usize>` — index of the crate a Rust path starts in.
- `pub fn by_name(&self, name: &str) -> Option<&Unit>` — public method, returns `Option<&Unit>`.
- `pub fn all_rust(&self) -> bool` — public method, returns `bool`.

## Depends on

- [[Unit]] — uses `Unit`, `is_rust`
- [[Language]] — uses `from_path`
- [[Symbol]] — uses `Symbol`

## Used by

- [[RenderCtx]] — from `render_crate`, `render_crate_flow`, `render_home`, `render_operations`
- [[Repo]] — from `Repo`, `build`, `top_modules`, `unit_index`
- [[Plan Module]] — from `plan`
- [[Units Module]] — from `discover`

## See also

- [[_index]]
- [[Units Module]]
