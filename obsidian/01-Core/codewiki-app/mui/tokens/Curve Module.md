---
tags: [core, codewiki-app, module]
---

# Curve Module

> The control points of MUI's easing curves.

## What it does

`createTransitions.js`, `easing`.

## Source

- `crates/codewiki-app/src/mui/tokens.rs` — module `codewiki_app::mui::tokens::curve`

## Items

- `pub const EASE_IN_OUT: [f32; 4]` — public constant; used by `Base::new`, `Checkbox::render`, `IconButton::render`, `ListFolder::render` and 6 more.
- `pub const EASE_OUT: [f32; 4]` — public constant; used by `Base::new`, `Select::render`, `TextField::render`, `drawer`.
- `pub const EASE_IN: [f32; 4]` — public constant.
- `pub const SHARP: [f32; 4]` — public constant; used by `drawer`.

## Used by

- [[Button Module]] — from `ease`, `render`
- [[Doc View Module]] — from `table_row`
- [[Base Module]] — from `new`
- [[Checkbox Module]] — from `render`
- [[List Module]] — from `render`
- [[Motion Module (mui)]] — from `drawer`
- [[Ripple Module]] — from `frame`
- [[Select Module]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]
