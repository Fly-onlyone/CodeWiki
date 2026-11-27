---
tags: [core, cw-wiki]
---

# Units

> Every unit of a repository and the lookup from code to unit.

## What it does

The list of all [[Unit]]s of a repository, plus two lookup tables that map code to a unit. Given a symbol, the planner and the renderer use it to say which crate or package the symbol belongs to.

## Source

- `crates/cw-wiki/src/units.rs` — definition of `Units`

## How it works

For a Rust symbol, `index_of` takes the first segment of its qualified name (the crate name) and looks it up with `rust_index`. For code in any other language it looks up the symbol's source file in a file map instead. `by_name` finds a unit by its stable key. `all_rust` is true when no unit is in another language. The renderer uses that to choose the words crate or package.

## Items

Declared as `pub struct Units`.

Members:

- `pub fn c_tests(&self) -> bool` — true when the repository has tests written in C or C++.
- `pub fn index_of(&self, symbol: &Symbol) -> Option<usize>` — index of the unit that holds `symbol`.
- `pub fn rust_index(&self, qualified: &str) -> Option<usize>` — index of the crate a Rust path starts in.
- `pub fn is_test(&self, file: &str) -> bool` — true for a source file the units found to be test code: one that holds a test, or sits in a project of tests.
- `pub fn of_file(&self, file: &str) -> Option<&Unit>` — unit of a source file outside Rust.
- `pub fn by_name(&self, name: &str) -> Option<&Unit>` — public method, returns `Option<&Unit>`.
- `pub fn all_rust(&self) -> bool` — public method, returns `bool`.

## Depends on

- [[Unit]] — uses `Unit`, `is_rust`
- [[Language]] — uses `from_path`
- [[Symbol]] — uses `Symbol`

## Used by

- [[RenderCtx]] — from `entry_label`, `render_crate`, `render_crate_flow`, `render_home`
- [[Repo]] — from `Repo`, `build`, `top_modules`, `unit_index`
- [[Plan Module]] — from `plan`
- [[Units Module]] — from `discover`

## Gotchas

- A symbol in a file that belongs to no unit gives `None`.
- `all_rust` is also true for an empty list.
- The two lookup maps are private and filled only by `discover`.

## See also

- [[_index]]
- [[Units Module]]
