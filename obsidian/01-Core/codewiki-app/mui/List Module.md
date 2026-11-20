---
tags: [core, codewiki-app, module]
---

# List Module

> `List`: rows that can be pressed, a heading over a group of them, and a folder that opens and closes.

## What it does

Row anatomy is MUI's `ListItemButton` in a dense list (`ListItemButton/ListItemButton.js`, `ListSubheader/ListSubheader.js`). How a row answers the pointer is TradingAgent's navigation (`components/layout/Sidebar.tsx`): it shifts four pixels to the right, its icon grows and takes the primary color, and the selected row carries a glowing bar on its left edge. The bar is in the theme's tertiary color, so that it stands out from the row's primary fill. A row may instead give its icon a hue of its own, which it keeps in every state, so that kinds of row stay apart.

## Source

- `crates/codewiki-app/src/mui/list.rs` — module `codewiki_app::mui::list`

## How it works

1. `AppView::sidebar` in [[codewiki-app]] and `Menu::render` in [[Menu Module]] call `subheader`. `subheader` calls `px` in [[Zoom Module]], then `brand` in [[Themes Module]].
2. `ListItemButton::render` in [[ListItemButton]] calls `fill`.
3. `ListItemButton::render` in [[ListItemButton]] calls `bar_glow`. `bar_glow` calls `px` in [[Zoom Module]].
4. `ListItemButton::render` in [[ListItemButton]] calls `icon_color`.
5. `AppView::sidebar` in [[codewiki-app]] calls `ListFolder::new`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const ROW_HEIGHT: f32` — A dense `ListItemButton`: 4 px above and below a 14 px label with MUI's 4 px text margins.
- `const TWO_LINE_HEIGHT: f32` — A dense row with MUI's `secondary` text under its label, at 12 px.
- `const INSET: f32` — `ListItemButton.js`: `paddingLeft: 16`. Half of it is the list's own inset here, so the row's fill stops short of the edges.
- `const INDENT: f32` — each level of nesting; MUI's examples indent by `pl: 4`.
- `const FONT: f32` — `Sidebar.tsx`: `fontSize: "0.875rem"`, a small icon.
- `const SHIFT: f32` — `Sidebar.tsx`: `translateX(4px)` and an icon at 115% under the pointer.
- `const BAR: (f32, f32)` — `Sidebar.tsx`: the bar of the selected row, 3 by 24 px.
- `const SUBHEADER_HEIGHT: f32` — `ListSubheader.js`: `lineHeight: '48px'`, 14 px text.
- `fn fill(primary: Hsla, selected: bool, hovered: bool) -> Hsla` — `Sidebar.tsx`: how strongly the primary color fills a row.
- `fn bar_glow(primary: Hsla) -> Vec<BoxShadow>` — `Sidebar.tsx`: the glow of the selected row's bar, in the bar's color.
- `fn icon_color(tint: Option<Hsla>, primary: Hsla, muted: Hsla, active: bool) -> Hsla` — the color of a row's icon: its own hue when it has one, else muted at rest and primary when the row is active.
- `pub fn subheader(label: impl Into<SharedString>, cx: &App) -> Div` — the heading over a group of rows, in the theme's secondary color, which is its link color.
- [[ListItemButton]] — A row that can be pressed.
- `pub struct ListFolder` — A row that opens and closes the rows under it.

And 5 more private items with no comment.

## Depends on

- [[ListItemButton]] — uses `ListItemButton`, `icon`, `new`, `on_click`
- [[Motion Module (mui)]] — uses `scaled`
- [[Tokens Module]] — uses `easing`
- [[Curve Module]] — uses `EASE_IN_OUT`
- [[Duration Module]] — uses `STANDARD`
- [[Themes Module]] — uses `brand`
- [[Zoom Module]] — uses `px`
- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `sidebar`
- [[Menu Module]] — from `render`
- [[ListItemButton]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]
