---
tags: [core, codewiki-app]
---

# Menu

> A menu that hangs under `anchor`.

## Source

- `crates/codewiki-app/src/mui/menu.rs` — definition of `Menu`

## How it works

Declared as `pub struct Menu`.

Public API:

- `pub fn new(id: impl Into<SharedString>, anchor: Bounds<Pixels>, focus: &FocusHandle) -> Self` — `focus` is the handle the menu takes its keys through; give it the focus when the menu opens.
- `pub fn on_highlight( mut self, on_highlight: impl Fn(usize, &mut Window, &mut App) + 'static, ) -> Self` — called with the row a key moved the highlight to.
- `pub fn on_close(mut self, on_close: impl Fn(&mut Window, &mut App) + 'static) -> Self` — called on Escape and on a click outside the menu.
- `pub fn on_select(mut self, on_select: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self` — called with a row's place when it is clicked or chosen with a key.
- `pub fn highlighted(mut self, highlighted: usize) -> Self` — the row the keys act on, counted through every section.
- `pub fn open(mut self, open: bool) -> Self` — whether the menu takes input.

## Depends on

- [[Menu Module]] — uses `ELEVATION`, `GAP`, `HEIGHT_INSET`, `LIST_PADDING`
- [[ListItemButton]] — uses `icon`, `new`, `trailing`
- [[Doc View Module]] — uses `id`
- [[List Module]] — uses `subheader`
- [[Motion Module (mui)]] — uses `enter`
- [[Surfaces Module]] — uses `paper`
- [[Enter]] — uses `MENU`

## Used by

- [[codewiki-app]] — from `theme_menu_view`

## See also

- [[_index]]
- [[Menu Module]]
