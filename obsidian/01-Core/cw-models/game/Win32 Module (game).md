---
tags: [core, cw-models, module]
---

# Win32 Module (game)

> Module `cw_models::game::win32` defining 3 items, including `full_screen`, `load_3d_of_others`, and `Query`.

## Source

- `crates/cw-models/src/game.rs` — module `cw_models::game::win32`

## How it works

1. `busy` in [[Game Module]] calls `full_screen`.
2. `busy` in [[Game Module]] calls `load_3d_of_others`.

## Items

- `pub fn full_screen() -> bool` — public function, returns `bool`; called by `busy`.
- `pub fn load_3d_of_others() -> Option<f64>` — public function, returns `Option<f64>`; called by `busy`.
- `struct Query` — closes the query when it goes out of scope, on every path.

## Used by

- [[Game Module]] — from `busy`

## See also

- [[_index]]
- [[cw-models]]
