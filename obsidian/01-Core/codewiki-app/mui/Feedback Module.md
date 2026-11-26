---
tags: [core, codewiki-app, module]
---

# Feedback Module

> `Alert`, `Chip`, `LinearProgress` and the tooltip: how the app tells the user something.

## What it does

This module holds the small pieces the app uses to tell the user something: a tinted `alert` box with an icon, a `chip` label, color `dots`, a `tooltip`, and two progress bars. They are plain functions that return GPUI elements, so a page can drop one in without a component of its own.

The sizes copy Material UI (MUI). The colors and corners follow the TradingAgent theme. Every size goes through the zoom module's `px`, so the pieces follow the app zoom.

## Source

- `crates/codewiki-app/src/mui/feedback.rs` — module `codewiki_app::mui::feedback`

## How it works

[[Tone]] says what kind of news a piece is: neutral, tertiary, success, info, warning or error. `Tone::color` reads the matching theme color, and `Tone::icon` picks the alert icon.

`alert_titled` builds a row: a 22 px icon in the tone color, then a column with an optional medium-weight title and the message. The box is the tone at 15% opacity with the theme's corner. `alert` is the same without a title. `alert_action` is a slot at the end of the row for one small button.

`chip` is 24 px high. Outlined chips use the tone at 70% for the border; a neutral one uses the foreground at 23%. A filled chip uses the tone at 15% instead of a border. The corner is the theme radius minus 2, at least 4.

`progress` draws a determinate bar filled to a share between 0 and 1. `linear_progress` is the indeterminate one: `indeterminate` gives the left and right edge of two bars for a point in a 2.1 s pass, using MUI's two keyframe curves, and the second bar starts 1.15 s later. An animation repeats it.

`tooltip` returns a builder to pass to GPUI's tooltip method.

## Items

- [[Tone]] — what kind of news something is.
- `const ALERT_FILL: f32` — `MuiAlert`, `standard`: the tone at 15% behind text in the tone's color.
- `const ALERT_PADDING: (f32, f32)` — `Alert.js`: `padding: '6px 16px'`, a 22 px icon 12 px before the message, which has 8 px above and below it.
- `const ALERT_TITLE_FONT: f32` — `AlertTitle.js`: `body1`, in medium weight.
- `pub fn alert(tone: Tone, message: impl Into<SharedString>, cx: &App) -> Div` — A message in a tinted box with an icon: MUI's standard `Alert`.
- `alert_titled` — an `alert` with MUI's `AlertTitle` above the message: the title in medium weight, at the size of body text.
- `const ALERT_ACTION_PADDING: (f32, f32)` — `Alert.js`, `AlertAction`: `padding: '4px 0 0 16px'`, `marginRight: -8`.
- `pub fn alert_action(action: impl IntoElement) -> Div` — the slot at the end of an `alert` that holds its one action, such as a small text button.
- `const CHIP_SMALL: (f32, f32)` — `Chip.js`, `size: 'small'`: 24 px high, with 8 px beside a 13 px label.
- `const CHIP_RADIUS_INSET: f32` — `MuiChip`: `borderRadius: max(br - 2, 4)`.
- `const CHIP_FILL: f32` — `MuiChip`, `filled`: the color at 15% behind the label.
- `pub fn chip(label: impl Into<SharedString>, tone: Tone, filled: bool, cx: &App) -> Div` — A small label in a rounded box, at MUI's small size. Outlined, as most of TradingAgent's are, unless `filled`.
- `const DOT: f32` — A dot of a color sample, and the space between two.
- `pub fn dots(colors: impl IntoIterator<Item = Hsla>, cx: &App) -> Div` — A row of color samples, such as the colors of a theme. Each has a thin ring, so that a sample in the color of the surface under it still shows.
- `const PROGRESS_HEIGHT: f32` — `LinearProgress.js`: a 4 px track; `MuiLinearProgress`: corners of 4 px.
- `pub fn indeterminate(time: f32) -> [(f32, f32); 2]` — where the two bars of an indeterminate progress are, `time` of the way through a pass: for each, the left edge and the right edge as shares of the track. `LinearProgress.js`, `indeterminate1Keyframe` and `indeterminate2Keyframe`, the second starting 1.15 s after the first.
- `pub fn linear_progress(id: &'static str, cx: &App) -> impl IntoElement + use<>` — A thin bar that shows work is going on, without saying how much is left.
- `const TOOLTIP_FONT: f32` — `MuiTooltip`: 12 px text with 6 by 12 px of padding and corners of 6 px.
- `pub fn tooltip(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView` — builds the tooltip a control shows: pass to GPUI's `.tooltip(…)`.
- `pub fn progress(share: f32, color: Hsla, cx: &App) -> Div` — A thin bar filled to `share` (0 to 1) in `color`: MUI's determinate `LinearProgress`.

And 7 more items.

## Depends on

- [[Tone]] — uses `Tone`, `color`, `icon`
- [[Motion Module (mui)]] — uses `scaled`
- [[Tokens Module]] — uses `easing`
- [[Zoom Module]] — uses `px`

## Used by

- [[codewiki-app]] — from `body`, `graph_view`, `page_view`, `reconnect_banner`
- [[Doc View Module]] — from `draw`, `draw_cell`
- [[Illustration Module]] — from `render`
- [[Button Module]] — from `render`

## Gotchas

- With reduced motion on, `linear_progress` shows a still bar, a quarter to three quarters of the track. It does not animate.
- The pass length goes through `scaled`, so the motion setting changes the animation speed.
- `indeterminate` edges can lie outside 0 to 1 (a bar starts off the left edge). `linear_progress` clamps them and skips a bar whose right edge is not past its left edge.
- `chip` takes the label color from the tone, except for `Neutral`, which uses the plain foreground.
- `Info`, `Neutral` and `Tertiary` alerts share the same info icon.
- `dots` gives each sample a thin ring, so a color equal to the surface still shows.

## See also

- [[_index]]
- [[codewiki-app]]
