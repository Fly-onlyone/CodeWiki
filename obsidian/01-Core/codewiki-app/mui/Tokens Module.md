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

The module is a table of constants plus a few small functions that turn them into GPUI values.

- `duration` and `curve` (see [[Duration Module]] and [[Curve Module]]) hold MUI's transition times and easing control points. `easing` turns four control points into a function from time share to progress share.
- `states` returns the hover, selected, focus, activated and disabled opacities. A dark theme tints more strongly than a light one.
- `elevation` returns the three box shadows (umbra, penumbra, ambient) for a level from 0 to 24. `elevation_between` blends two levels by a progress value, for a surface that rises or settles. The blur is clamped to zero because a spring can overshoot. Sizes go through the zoom module's `px`.
- `Variant` lists MUI's thirteen text styles, and `Variant::spec` gives the size, weight, line height and capitals flag of each. These are TradingAgent's values, which override MUI's.
- `SPACING` is one step, 8 px, of MUI's spacing scale.

## Items

- `pub const SPACING: f32` — one step of MUI's spacing scale (`theme.spacing(1)`).
- `pub fn easing(points: [f32; 4]) -> impl Fn(f32) -> f32` — an easing function for one of the `curve` constants.
- `pub struct StateOpacity` — how strongly a state tints a control. `styles/createPalette.js`, the `action` object of the light and the dark palette.
- `pub const LIGHT_STATES: StateOpacity` — public constant; used by `LIGHT_RATIO`, `states`.
- `pub const DARK_STATES: StateOpacity` — public constant; used by `LIGHT_RATIO`, `states`.
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

- [[Doc View Module]] — from `line`, `render`, `stat_tile`, `table_row`
- [[Button Module]] — from `ease`, `line`, `render`
- [[Surfaces Module]] — from `card_content`, `paper`, `text`
- [[Checkbox Module]] — from `of`, `render`
- [[Motion Module (mui)]] — from `drawer`, `glide`
- [[Base Module]] — from `new`
- [[Feedback Module]] — from `indeterminate`
- [[Interaction Module]] — from `LIGHT_RATIO`

## Gotchas

- Where MUI and TradingAgent differ, TradingAgent wins. For example, the button style is not in capitals and is semibold.
- Levels above 24 draw as 24, so a bad level is safe.
- The `ELEVATION` table has `rustfmt::skip`, so reformatting does not scramble the 25 rows.
- Elevation is a function of zoom. Do not cache the shadow list across a zoom change.
- `Overline` is the only variant with uppercase set; `text` in [[Surfaces Module]] applies it.
- A comment on each constant names its MUI source file. Keep it when you change a value, so the next reader can check it.

## See also

- [[_index]]
- [[codewiki-app]]
