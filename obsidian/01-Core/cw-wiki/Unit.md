---
tags: [core, cw-wiki]
---

# Unit

> One top-level part of the repository.

## Source

- `crates/cw-wiki/src/units.rs` — definition of `Unit`

## How it works

Declared as `pub struct Unit`.

Public API:

- `pub fn declares(&self, package: &str) -> Vec<&str>` — true when a build file of the unit mentions `package` as a whole word.
- `pub fn is_rust(&self) -> bool`

## Depends on

- [[Units Module]] — uses `Entry`, `Manifest`
- [[Language]] — uses `Language`

## Used by

- [[RenderCtx]] — from `illustration`, `render_crate`, `render_integration`, `render_operations`
- [[Repo]] — from `module_count`, `top_modules`, `unit_of`
- [[Units]] — from `Units`, `all_rust`, `by_name`
- [[Plan Module]] — from `module_dir`, `plan`
- [[Units Module]] — from `discover`

## See also

- [[_index]]
- [[Units Module]]
