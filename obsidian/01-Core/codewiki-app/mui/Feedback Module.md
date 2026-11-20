---
tags: [core, codewiki-app, module]
---

# Feedback Module

> `Alert`, `Chip`, `LinearProgress` and the tooltip: how the app tells the user something.

## Source

- `crates/codewiki-app/src/mui/feedback.rs` — module `codewiki_app::mui::feedback`

## How it works

Main items:

- [[Tone]] — what kind of news something is.
- `alert` — A message in a tinted box with an icon: MUI's standard `Alert`.
- `chip` — A small label in a rounded box, at MUI's small size.
- `indeterminate` — where the two bars of an indeterminate progress are, `time` of the way through a pass: for each, the left edge and the right edge as shares of the track.
- `progress` — A thin bar filled to `share` (0 to 1) in `color`: MUI's determinate `LinearProgress`.
- `linear_progress` — A thin bar that shows work is going on, without saying how much is left.
- `dots` — A row of color samples, such as the colors of a theme.
- `tooltip` — builds the tooltip a control shows: pass to GPUI's `.tooltip(…)`.

## Depends on

- [[Tone]] — uses `Tone`, `color`, `icon`
- [[Motion Module (mui)]] — uses `scaled`
- [[Tokens Module]] — uses `easing`

## Used by

- [[codewiki-app]] — from `body`, `graph_view`, `page_view`, `settings_frame`
- [[Doc View Module]] — from `draw`, `draw_cell`
- [[Illustration Module]] — from `render`
- [[Button Module]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]
