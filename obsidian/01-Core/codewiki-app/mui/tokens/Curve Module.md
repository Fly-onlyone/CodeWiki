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

- `pub const EASE_IN_OUT: [f32; 4]` — public constant; used by `IconButton::render`, `ListFolder::render`, `ListItemButton::render`, `Switch::render` and 3 more.
- `pub const EASE_OUT: [f32; 4]` — public constant; used by `Select::render`, `TextField::render`, `drawer`.
- `pub const EASE_IN: [f32; 4]` — public constant.
- `pub const SHARP: [f32; 4]` — public constant; used by `drawer`.

## Used by

- [[Button Module]] — from `ease`, `render`
- [[Doc View Module]] — from `table_row`
- [[List Module]] — from `render`
- [[Motion Module (mui)]] — from `drawer`
- [[Ripple Module]] — from `frame`
- [[Select Module]] — from `render`
- [[Switch Module]] — from `render`
- [[Text Field Module]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]
