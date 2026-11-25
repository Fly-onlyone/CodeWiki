---
tags: [core, codewiki-app, module]
---

# Interaction Module

> How strongly a control answers the pointer: one set of numbers for hover, press and selection, read by every control in `mui`.

## What it does

One place for how strongly every control answers the pointer. Hover, press and selection strengths live in one `Interaction` struct, so the buttons, rows, tabs, fields and links of the app react in a matching way and can be tuned together.

The values in `DARK` are stronger than MUI's own and TradingAgent's, because those were too faint on the app's glass. The owner chose them by eye on 2026-11-25. Fills are opacities of the control's color, lifts are changes of lightness, lengths are in pixels. The struct also holds the list-row shift and recoil, the selected bar's width and spring, the ripple opacities, and the link hover look (lift, underline opacity, thickness).

## Source

- `crates/codewiki-app/src/mui/interaction.rs` — module `codewiki_app::mui::interaction`

## How it works

`DARK` is the only table of numbers. `Interaction::for_mode` turns it into the values for a light theme: state fills (hover, pressed, selected, toggled, tab fills) are multiplied by `LIGHT_RATIO`, which is the light hover over the dark hover from the token table (0.5). Every lift (icon, label, icon button, toggled icon, link) is negated, so a light theme darkens where a dark one brightens. Row fills, lengths, springs and ripple opacities stay as they are.

`current(cx)` reads the active theme and returns the right set. Every `mui` control calls it at render time, so a theme switch takes effect at once.

Two helpers do the color work. `lift` changes only lightness and clamps it to 0..1, so a tinted icon keeps its hue and saturation. `pressed` raises a fill's alpha toward the pressed alpha by a progress from 0 to 1; it never lowers it.

`Accent` names which theme color a selected or switched-on control takes: `Primary`, or `Secondary`, which is the theme's link color.

## Items

- `pub const LIGHT_RATIO: f32` — what a light theme's state fills are of a dark theme's.
- `pub enum Accent` — A color a selected or switched-on control takes, from the active theme.
- `pub struct Interaction` — the strengths. Fills are opacities of the control's color, lifts are changes of lightness, lengths are in pixels.
- `pub const DARK: Interaction` — the values for a dark theme, chosen by eye on 2026-11-25.
- `pub fn current(cx: &App) -> Interaction` — the values in effect for the active theme.
- `pub fn lift(color: Hsla, amount: f32) -> Hsla` — `color` made `amount` lighter, or darker for a negative amount. Hue and saturation stay, so a tinted icon keeps its kind.
- `pub fn pressed(rest: Hsla, alpha: f32, progress: f32) -> Hsla` — `rest` on its way to the pressed fill `alpha`, `progress` of the way. A fill already stronger than the pressed one stays as it is.

## Depends on

- [[Tokens Module]] — uses `DARK_STATES`, `LIGHT_STATES`
- [[Motion Module (mui)]] — uses `BOUNCY`
- [[Spring]] — uses `Spring`

## Used by

- [[List Module]] — from `fill`, `icon_color`, `render`
- [[Button Module]] — from `glow`, `render`
- [[Base Module]] — from `attach`
- [[Checkbox Module]] — from `of`
- [[Select Module]] — from `render`
- [[Switch Module]] — from `render`
- [[Tabs Module]] — from `render`
- [[Text Field Module]] — from `render`

## Gotchas

- Edit `DARK` only. Light values are derived, so a number added to the struct must also be handled in `for_mode`, or it stays the same in both modes.
- Row fills do not halve in light mode. Only the state fills do.
- A lift is signed. Add a new lift field to the negated list in `for_mode`, or light themes will brighten instead of darken.
- `pressed` leaves a fill that is already stronger than the pressed alpha untouched, so a press never weakens a selected control.
- Tests pin the order of strengths (row hover < selected < selected hover < pressed). A retune that breaks it fails a test.

## See also

- [[_index]]
- [[codewiki-app]]
