---
tags: [core, codewiki-app, module]
---

# Feedback Module

> `Alert`, `Chip`, `LinearProgress` and the tooltip: how the app tells the user something.

## What it does

Sizes are MUI's (`Alert/Alert.js`, `Chip/Chip.js`, `LinearProgress/LinearProgress.js`, `Tooltip/Tooltip.js`); colors and corners are TradingAgent's (`components.ts`).

## Source

- `crates/codewiki-app/src/mui/feedback.rs` — module `codewiki_app::mui::feedback`

## How it works

1. `AppView::page_view` and `AppView::viewer` in [[codewiki-app]], and 2 more, call `alert`. `alert` calls `alert_titled`.
2. `AppView::graph_view` and `title_bar` in [[codewiki-app]], and 2 more, call `chip`. `chip` calls `px` in [[Zoom Module]].
3. `AppView::body` in [[codewiki-app]] and `draw` in [[Doc View Module]] call `linear_progress`. `linear_progress` calls `px` in [[Zoom Module]], then `scaled` in [[Motion Module (mui)]], then `indeterminate`. `indeterminate` calls `easing` in [[Tokens Module]].
4. `render` in [[Illustration Module]] calls `alert_titled`. `alert_titled` calls `px` in [[Zoom Module]].
5. `draw` in [[Doc View Module]] calls `alert_action`. `alert_action` calls `px` in [[Zoom Module]].

3 more entry points: `dots`, `tooltip`, `progress`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

- [[codewiki-app]] — from `body`, `graph_view`, `page_view`, `settings_frame`
- [[Doc View Module]] — from `draw`, `draw_cell`
- [[Illustration Module]] — from `render`
- [[Button Module]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]
