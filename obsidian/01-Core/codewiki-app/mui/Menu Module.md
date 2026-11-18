---
tags: [core, codewiki-app, module]
---

# Menu Module

> `Menu`: a short list of choices that opens beside the control that asked for it.

## Source

- `crates/codewiki-app/src/mui/menu.rs` — module `codewiki_app::mui::menu`

## How it works

Main items:

- `Menu` — A menu that hangs under `anchor`.
- `MenuItem` — one choice.
- `MenuSection` — A group of choices, under a heading when it has one.
- `AnchorBounds` — where a control is in the window, so that a menu can open beside it.
- `Align` — which edge of its anchor a menu lines up with.
- `Key` — A key that moves the highlight.
- `step` — the row the highlight moves to from `current` in a menu of `count` rows.
- `WIDTH` — MUI sizes a menu to its widest row.

## Depends on

- [[ListItemButton]] — uses `icon`, `new`, `trailing`
- [[Motion Module (mui)]] — uses `MENU`, `enter`
- [[Doc View Module]] — uses `set`
- [[List Module]] — uses `subheader`
- [[Surfaces Module]] — uses `paper`
- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `AppView`, `settings_menu_view`, `theme_menu_item`, `theme_menu_view`
- [[Doc View Module]] — from `select_control`
- [[Select Module]] — from `menu`

## See also

- [[_index]]
- [[codewiki-app]]
