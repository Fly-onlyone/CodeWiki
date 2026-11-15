---
tags: [core, codewiki-app, module]
---

# Menu Module

> `Menu`: a short list of choices that opens beside the control that asked for it.

## Source

- `crates/codewiki-app/src/mui/menu.rs` — module `codewiki_app::mui::menu`

## How it works

Main items:

- [[Menu]] — A menu that hangs under `anchor`.
- `MenuItem` — one choice.
- `MenuSection` — A group of choices, under a heading when it has one.
- `AnchorBounds` — where a control is in the window, so that a menu can open beside it.
- `Key` — A key that moves the highlight.
- `step` — the row the highlight moves to from `current` in a menu of `count` rows.
- `WIDTH` — MUI sizes a menu to its widest row.
- `OnClose` — type alias

## Depends on

- [[Doc View Module]] — uses `set`
- [[Glyph]] — uses `Glyph`

## Used by

- [[Menu]] — from `Menu`, `new`, `render`, `section`
- [[codewiki-app]] — from `AppView`, `theme_menu_view`, `toolbar`

## See also

- [[_index]]
- [[codewiki-app]]
