---
tags: [core, codewiki-app, module]
---

# Tokens Module

> The numbers behind the components.

## Source

- `crates/codewiki-app/src/mui/tokens.rs` — module `codewiki_app::mui::tokens`

## How it works

Main items:

- `Variant` — A text style.
- `TextSpec` — what a `Variant` comes to.
- `easing` — an easing function for one of the `curve` constants.
- `StateOpacity` — how strongly a state tints a control.
- `elevation_between` — A shadow part of the way from one elevation to another, for a surface that is rising or settling.
- `SPACING` — one step of MUI's spacing scale (`theme.spacing(1)`).
- `elevation` — the shadow of a surface raised to `level` (0 to 24).
- `states` — the state opacities for a dark or a light theme.

Submodules: [[Curve Module]], [[Duration Module]].

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
