---
tags: [core, codewiki-app, module]
---

# Curve Module

> The control points of MUI's easing curves.

## What it does

These are the four easing curves of Material UI, copied from its transitions file. An easing curve says how fast an animation moves at each moment: slow start, slow end, or both. Each curve is four numbers, the control points of a cubic Bezier. Components pick one by name so that every control moves in the same way.

## Source

- `crates/codewiki-app/src/mui/tokens.rs` — module `codewiki_app::mui::tokens::curve`

## How it works

Pass a curve to `tokens::easing`. It returns a function that maps the share of time passed (0 to 1) to the share of the change done (0 to 1). A transition then uses that function.

- `EASE_IN_OUT` starts and ends slowly. Checkbox, switch, icon buttons and list rows use it for color and position changes.
- `EASE_OUT` starts fast and ends slowly. Text fields, selects and the drawer's open step use it.
- `EASE_IN` starts slowly and ends fast.
- `SHARP` is a quick curve for something that may return, such as the drawer closing.

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

## Gotchas

- Pair a curve with a duration from [[Duration Module]] and pass the time through `motion::scaled`, so the user's motion setting is respected.
- The values are MUI's. Changing one changes every control that uses it. A test checks that every curve starts at 0 and ends at 1, and that ease-out is ahead of ease-in at the half way point.
- `EASE_IN` has no user in the app today.

## See also

- [[_index]]
- [[codewiki-app]]
