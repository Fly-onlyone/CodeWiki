---
tags: [core, codewiki-app, module]
---

# Tokens Module

> The numbers behind the components.

## What it does

Each one is copied from Material UI 6.5 or from the TradingAgent theme that restyles it, and names the file it came from. Where the two differ, the TradingAgent value is used. Sizes are in pixels at MUI's 16 px root.

MUI paths are relative to `@mui/material`; TradingAgent paths to its `frontend/src/theme`.

## Source

- `crates/codewiki-app/src/mui/tokens.rs` — module `codewiki_app::mui::tokens`

## How it works

1. `table_row` in [[Doc View Module]] and `IconButton::render` in [[Button Module]], and 9 more, call `easing`.
2. `Button::render` and `IconButton::render` in [[Button Module]], and 4 more, call `states`.
3. `paper` in [[Surfaces Module]] and `Switch::render` in [[Switch Module]] call `elevation`. `elevation` calls `elevation_between`. `elevation_between` calls `black`, then `px` in [[Zoom Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const SPACING: f32` — one step of MUI's spacing scale (`theme.spacing(1)`).
- `pub fn easing(points: [f32; 4]) -> impl Fn(f32) -> f32` — an easing function for one of the `curve` constants.
- `pub struct StateOpacity` — how strongly a state tints a control. `styles/createPalette.js`, the `action` object of the light and the dark palette.
- `pub const LIGHT_STATES: StateOpacity` — public constant; used by `states`.
- `pub const DARK_STATES: StateOpacity` — public constant; used by `states`.
- `pub fn states(dark: bool) -> StateOpacity` — the state opacities for a dark or a light theme.
- `const UMBRA: f32` — opacity of the three layers of an elevation shadow. `styles/shadows.js`.
- `pub const MAX_ELEVATION: usize` — the highest elevation MUI defines.
- `const ELEVATION: [[i8; 12]; MAX_ELEVATION + 1]` — `styles/shadows.js`: for each elevation, three shadows of (x, y, blur, spread).
- `pub fn elevation(level: usize) -> Vec<BoxShadow>` — the shadow of a surface raised to `level` (0 to 24). Levels above the highest are drawn as the highest.
- `pub fn elevation_between(from: usize, to: usize, progress: f32) -> Vec<BoxShadow>` — A shadow part of the way from one elevation to another, for a surface that is rising or settling. `progress` runs from 0 (`from`) to 1 (`to`) and may overshoot.
- `fn black(opacity: f32) -> Hsla` — private function, returns `Hsla`; called by `elevation_between`.
- `pub enum Variant` — A text style. `typography.ts` (TradingAgent), which overrides `styles/createTypography.js`.
- `pub struct TextSpec` — what a `Variant` comes to.

And 2 more private items with no comment.

Submodules: [[Curve Module]], [[Duration Module]].

## Depends on

- [[Zoom Module]] — uses `px`

## Used by

- [[Doc View Module]] — from `line`, `stat_tile`, `table_row`, `table_text`
- [[Button Module]] — from `ease`, `line`, `render`
- [[Surfaces Module]] — from `card_content`, `paper`, `text`
- [[Feedback Module]] — from `indeterminate`
- [[List Module]] — from `render`
- [[Motion Module (mui)]] — from `drawer`
- [[Ripple Module]] — from `frame`
- [[Select Module]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]
