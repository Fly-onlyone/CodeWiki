---
tags: [core, codewiki-app]
---

# ListItemButton

> A row that can be pressed.

## What it does

A pressable row for sidebars, menus, palettes and settings lists. You build it with chained setters: an icon, a tint, a selected flag, a nesting level, a second line, something at the far end, and a click handler. It lives in [[List Module]], which also explains the look and motion.

## Source

- `crates/codewiki-app/src/mui/list.rs` — definition of `ListItemButton`

## How it works

`new` takes an id and a label. The setters only store values. All work happens in `render`: it makes a `Base` for pointer state (see [[Base Module]]), reads the strengths from [[Interaction Module]], and builds the row from a fill, a hover spring and a bar spring. Press progress from `Base` raises the fill and pulls the row back part of its shift. `selected` turns on the bar, a semibold label and the accent color. `highlighted` counts as hovered, for the key-driven row of a menu. `secondary` makes the row taller to hold a second line. `level` indents by 16 px for each step.

## Items

Declared as `pub struct ListItemButton`.

Members:

- `pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self` — public method, returns `Self`; called by `AppView::palette_view`, `AppView::settings_entry`, `AppView::settings_frame`, `AppView::sidebar` and 4 more.
- `pub fn icon(mut self, icon: Glyph) -> Self` — public method, returns `Self`; called by `ListItemButton::render`, `Menu::render`.
- `pub fn tint(mut self, tint: Option<Hsla>) -> Self` — draw the icon in this color at rest, under the pointer and when selected, in place of the muted color that turns primary. The row's fill, bar and ripple stay primary.
- `pub fn selected(mut self, selected: bool) -> Self` — public method, returns `Self`; called by `ListItemButton::render`.
- `pub fn highlighted(mut self, highlighted: bool) -> Self` — draw the row as if the pointer were over it: the row the keys act on in a menu, MUI's `focusVisible`.
- `pub fn level(mut self, level: usize) -> Self` — how deeply the row is nested: 0 for a top row.
- `pub fn code(mut self, code: bool) -> Self` — set the label in the code font.
- `pub fn secondary(mut self, text: impl Into<SharedString>) -> Self` — A second, quieter line under the label: MUI's `secondary` text. The row grows to hold it.
- `pub fn trailing(mut self, trailing: impl IntoElement) -> Self` — something at the far end of the row, such as a folder's arrow.
- `on_click` — public method, returns `Self`.
- `fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement` — public method, returns `impl IntoElement`.

## Depends on

- [[List Module]] — uses `FONT`, `ICON`, `ICON_GAP`, `INDENT`
- [[Base Module]] — uses `Ink`, `OnClick`, `focus_ring`, `new`
- [[Interaction Module]] — uses `current`, `lift`, `pressed`
- [[Motion Module (mui)]] — uses `SNAPPY`, `scaled`
- [[Tokens Module]] — uses `easing`
- [[Curve Module]] — uses `EASE_IN_OUT`
- [[Duration Module]] — uses `SHORTEST`
- [[Themes Module]] — uses `of`

## Used by

- [[codewiki-app]] — from `palette_view`, `settings_entry`, `settings_frame`, `sidebar`
- [[List Module]] — from `ListFolder`, `render`, `rows`
- [[Doc View Module]] — from `draw_row`
- [[Autocomplete Module]] — from `render`
- [[Menu Module]] — from `render`

## Gotchas

- `tint` sets the icon hue only. The fill, bar and ripple stay primary.
- The id keys the row's springs and transitions, so it must be stable between frames.
- A row has no click effect without `on_click`, but it still shows hover and fill.

## See also

- [[_index]]
- [[List Module]]
