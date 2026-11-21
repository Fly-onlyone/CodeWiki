---
tags: [core, codewiki-app, module]
---

# Menu Module

> `Menu`: a short list of choices that opens beside the control that asked for it.

## What it does

Anatomy and behaviour are MUI's (`Menu/Menu.js`, `Popover/Popover.js`, `MenuList/MenuList.js`, `MenuItem/MenuItem.js`); TradingAgent changes none of it. The surface is a `Paper` at elevation 8 that hangs under its anchor and stays inside the window. A click outside it, or Escape, closes it. The arrow keys, Home and End move a highlight that wraps at the ends, and Enter or Space choose the highlighted row.

The owner keeps the state: whether the menu is open, which row is highlighted, and the focus handle the menu takes its keys through. Tab is bound by the kit's root, which moves the focus on; the owner closes the menu when its handle loses the focus, as MUI closes a menu on Tab.

Three things MUI does are left out. Its `Grow` entrance scales the paper, which GPUI cannot do, so the menu fades and drops into place instead. Its subheaders stick to the top while their group scrolls; GPUI has no sticky position. And typing a letter does not jump to a row.

## Source

- `crates/codewiki-app/src/mui/menu.rs` — module `codewiki_app::mui::menu`

## How it works

1. `AppView::theme_menu_view` in [[codewiki-app]] and `menu` in [[Select Module]] call `Menu::new`.
2. `AppView::settings_menu_view` and `theme_menu_item` in [[codewiki-app]] call `MenuItem::new`.
3. `AppView::theme_menu_view` and `AppView::settings_menu_view` in [[codewiki-app]] call `MenuSection::new`.
4. `AppView::settings_menu_view` in [[codewiki-app]] calls `MenuSection::plain`.

## Items

- `const ELEVATION: usize` — `Popover.js`: `elevation = 8`.
- `const WINDOW_MARGIN: f32` — `Popover.js`: `marginThreshold = 16`, the least distance to the window's edge.
- `const HEIGHT_INSET: f32` — `Menu.js`: `maxHeight: 'calc(100% - 96px)'`.
- `const LIST_PADDING: f32` — `List.js`: `paddingTop: 8, paddingBottom: 8`.
- `const GAP: f32` — between the anchor and the menu. MUI lays a menu over its anchor; here it hangs under it, so the control that opened it stays in view.
- `const WIDTH: f32` — MUI sizes a menu to its widest row. A row here is a flex line that truncates, so the menu has a width of its own.
- `type OnIndex = Rc<dyn Fn(usize, &mut Window, &mut App)>` — what a chosen row does: it is given the row's place in the menu.
- `pub enum Key` — A key that moves the highlight.
- `pub fn step(current: usize, count: usize, key: Key) -> usize` — the row the highlight moves to from `current` in a menu of `count` rows. `MenuList.js`: `disableListWrap = false`, so it wraps at both ends.
- `pub struct AnchorBounds` — where a control is in the window, so that a menu can open beside it. Wrap the control with `AnchorBounds::track`; its bounds are then known from the first frame it is drawn in.
- `pub struct MenuItem` — one choice.
- `pub struct MenuSection` — A group of choices, under a heading when it has one.
- `pub enum Align` — which edge of its anchor a menu lines up with.
- `pub struct Menu` — A menu that hangs under `anchor`.
- `struct Scroll` — the scroll position of a menu, and the row it last brought into view.

And 1 more private item with no comment.

## Depends on

- [[ListItemButton]] — uses `icon`, `new`, `trailing`
- [[Motion Module (mui)]] — uses `MENU`, `enter`
- [[Doc View Module]] — uses `set`
- [[List Module]] — uses `subheader`
- [[Surfaces Module]] — uses `paper`
- [[Zoom Module]] — uses `px`
- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `AppView`, `settings_menu_view`, `theme_menu_item`, `theme_menu_view`
- [[Doc View Module]] — from `select_control`
- [[Select Module]] — from `menu`

## See also

- [[_index]]
- [[codewiki-app]]
