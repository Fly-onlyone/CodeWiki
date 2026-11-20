---
tags: [core, codewiki-app]
---

# ListItemButton

> A row that can be pressed.

## Source

- `crates/codewiki-app/src/mui/list.rs` — definition of `ListItemButton`

## How it works

1. `AppView::palette_view` and `AppView::sidebar` in [[codewiki-app]], and 5 more, call `new`.

## Items

Declared as `pub struct ListItemButton`.

Members:

- `pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self` — public method, returns `Self`; called by `AppView::palette_view`, `AppView::settings_entry`, `AppView::settings_frame`, `AppView::sidebar` and 3 more.
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

- [[List Module]] — uses `BAR`, `FONT`, `ICON`, `ICON_GAP`
- [[Base Module]] — uses `Ink`, `OnClick`, `focus_ring`, `new`
- [[Motion Module (mui)]] — uses `SNAPPY`, `scaled`
- [[Tokens Module]] — uses `easing`
- [[Curve Module]] — uses `EASE_IN_OUT`
- [[Duration Module]] — uses `SHORTEST`
- [[Themes Module]] — uses `of`
- [[Zoom Module]] — uses `px`

## Used by

- [[codewiki-app]] — from `palette_view`, `settings_entry`, `settings_frame`, `sidebar`
- [[List Module]] — from `ListFolder`, `render`, `rows`
- [[Doc View Module]] — from `draw_row`
- [[Menu Module]] — from `render`

## See also

- [[_index]]
- [[List Module]]
