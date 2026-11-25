---
tags: [core, codewiki-app, module]
---

# List Module

> `List`: rows that can be pressed, a heading over a group of them, and a folder that opens and closes.

## What it does

Builds the lists of the app: the sidebar, menus, the command palette and settings pages. It gives three pieces: `ListItemButton`, a pressable row; `subheader`, a heading over a group of rows; and `ListFolder`, a row that opens and closes the rows under it.

Row size and spacing follow MUI's dense list. How a row answers the pointer follows TradingAgent's sidebar, at the strengths in [[Interaction Module]]. Under the pointer a row fills, shifts right and its icon grows. A selected row has a stronger fill, a semibold label in a lightened accent, and a glowing bar on its left edge that grows and slides in. A row can carry its own icon hue, which it keeps in every state.

## Source

- `crates/codewiki-app/src/mui/list.rs` — module `codewiki_app::mui::list`

## How it works

`ListItemButton::render` reads the strengths with `interaction::current`, then builds three motions from the row's id.

- Fill: `fill` picks an opacity of the primary color from the selected and hovered state. A transition eases it, then `pressed` raises it toward `row_pressed` by the press progress that `Base` keeps (see [[Base Module]]).
- Hover: a spring runs from 0 to 1 and drives the right shift, the icon growth and the icon lift. While pressed, its target drops by the recoil share, so the row gives part of the shift back.
- Bar: a second spring, with the knob's `bar_spring`, runs to 1 when selected. `bar_place` turns its value into the bar's left edge and height, so the bar grows from its middle while it slides in. The bar is drawn only while the spring is above 0.001, and its three-layer glow scales with the spring.

`icon_color` picks the icon color. A tinted icon keeps its hue and is only lightened by `lift`. An untinted icon is muted at rest, primary when hovered, and the selected accent when selected. A tinted icon of a selected row also gets a glow, drawn as the shadow of a clear disc behind it.

`ListFolder` keeps its open state under its id and eases a `shown` value. That value turns the arrow half a turn and drives `MotionReveal` for the rows. Closed and at rest, the rows are not built.

## Items

- `pub const ROW_HEIGHT: f32` — A dense `ListItemButton`: 4 px above and below a 14 px label with MUI's 4 px text margins.
- `const TWO_LINE_HEIGHT: f32` — A dense row with MUI's `secondary` text under its label, at 12 px.
- `const INSET: f32` — `ListItemButton.js`: `paddingLeft: 16`. Half of it is the list's own inset here, so the row's fill stops short of the edges.
- `pub const ROW_START: f32` — where a top row's content starts, from the list's edge: what lines up with the rows, such as a heading's checkbox.
- `const INDENT: f32` — each level of nesting; MUI's examples indent by `pl: 4`.
- `const FONT: f32` — `Sidebar.tsx`: `fontSize: "0.875rem"`, a small icon.
- `const BAR_HEIGHT: f32` — `Sidebar.tsx`: the bar of the selected row is 24 px high. Its width, and how far a row shifts and its icon grows, are in `interaction`.
- `const BAR_TRAVEL: f32` — how far beyond its own width the bar starts when it slides in.
- `const SUBHEADER_HEIGHT: f32` — `ListSubheader.js`: `lineHeight: '48px'`, 14 px text.
- `fn fill(primary: Hsla, selected: bool, hovered: bool, look: &Interaction) -> Hsla` — how strongly the primary color fills a row.
- `fn bar_glow(color: Hsla, strength: f32) -> Vec<BoxShadow>` — `Sidebar.tsx`: the glow of the selected row's bar, in the bar's color, at `strength` times TradingAgent's.
- `fn icon_glow(tint: Hsla, opacity: f32) -> BoxShadow` — the glow around a selected row's tinted icon.
- `icon_color` — the color of a row's icon. One with a hue of its own keeps it, made `lifted` lighter. Else it is muted at rest, primary under the pointer, and the selected label's accent while selected.
- `fn bar_place(width: f32, shown: f32) -> (f32, f32)` — where the bar of a selected row is, `shown` of the way in: its left edge and its height, centred on the row. It grows from its middle as it slides in from the left.
- `pub fn subheader(label: impl Into<SharedString>, cx: &App) -> Div` — the heading over a group of rows, in the theme's secondary color, which is its link color.
- `pub struct ListItemButton` — A row that can be pressed.
- `pub struct ListFolder` — A row that opens and closes the rows under it.

And 4 more private items with no comment.

## Depends on

- [[Base Module]] — uses `Ink`, `OnClick`, `focus_ring`, `new`
- [[Interaction Module]] — uses `Interaction`, `current`, `lift`, `pressed`
- [[Motion Module (mui)]] — uses `SNAPPY`, `scaled`
- [[Duration Module]] — uses `SHORTEST`, `STANDARD`
- [[Themes Module]] — uses `brand`, `of`
- [[Tokens Module]] — uses `easing`
- [[Curve Module]] — uses `EASE_IN_OUT`
- [[Zoom Module]] — uses `px`

## Used by

- [[codewiki-app]] — from `palette_view`, `review_queue`, `settings_entry`, `settings_frame`
- [[Doc View Module]] — from `draw_row`
- [[Autocomplete Module]] — from `render`
- [[Menu Module]] — from `render`

## Gotchas

- The bar spring starts at its target, so a row that first appears selected shows its bar at once, with no slide.
- The bar spring may overshoot. `bar_place` clamps the height to 0..1 of the bar height, so the bar never goes negative or taller than the row.
- The bar uses the theme's tertiary color, not primary, so it stands out from the primary fill. The row's fill and ripple stay primary.
- `highlighted` draws the row as hovered but does not trigger the press or ripple. Menus use it for the key-driven row.
- A `ListFolder` is open only at first as `open_at_first` says. After that the user's state wins, even if the code asks again.
- Row ids must be unique and stable: the springs and transitions are keyed on them.

## See also

- [[_index]]
- [[codewiki-app]]
