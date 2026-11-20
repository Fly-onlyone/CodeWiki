---
tags: [core, codewiki-app]
---

# ListItemButton

> A row that can be pressed.

## Source

- `crates/codewiki-app/src/mui/list.rs` — definition of `ListItemButton`

## How it works

Declared as `pub struct ListItemButton`.

Public API:

- `pub fn secondary(mut self, text: impl Into<SharedString>) -> Self` — A second, quieter line under the label: MUI's `secondary` text.
- `pub fn trailing(mut self, trailing: impl IntoElement) -> Self` — something at the far end of the row, such as a folder's arrow.
- `pub fn level(mut self, level: usize) -> Self` — how deeply the row is nested: 0 for a top row.
- `pub fn tint(mut self, tint: Option<Hsla>) -> Self` — draw the icon in this color at rest, under the pointer and when selected, in place of the muted color that turns primary.
- `pub fn highlighted(mut self, highlighted: bool) -> Self` — draw the row as if the pointer were over it: the row the keys act on in a menu, MUI's `focusVisible`.
- `pub fn code(mut self, code: bool) -> Self` — set the label in the code font.

## Depends on

- [[List Module]] — uses `BAR`, `FONT`, `ICON`, `ICON_GAP`
- [[Base Module]] — uses `Ink`, `OnClick`, `focus_ring`, `new`
- [[Motion Module (mui)]] — uses `SNAPPY`, `scaled`
- [[Tokens Module]] — uses `easing`
- [[Curve Module]] — uses `EASE_IN_OUT`
- [[Duration Module]] — uses `SHORTEST`
- [[Themes Module]] — uses `of`
- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `palette_view`, `settings_entry`, `settings_frame`, `sidebar`
- [[List Module]] — from `ListFolder`, `render`, `rows`
- [[Doc View Module]] — from `draw_row`
- [[Menu Module]] — from `render`

## See also

- [[_index]]
- [[List Module]]
