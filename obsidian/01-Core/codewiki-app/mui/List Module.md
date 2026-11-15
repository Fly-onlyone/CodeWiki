---
tags: [core, codewiki-app, module]
---

# List Module

> `List`: rows that can be pressed, a heading over a group of them, and a folder that opens and closes.

## Source

- `crates/codewiki-app/src/mui/list.rs` — module `codewiki_app::mui::list`

## How it works

Main items:

- [[ListItemButton]] — A row that can be pressed.
- `ListFolder` — A row that opens and closes the rows under it.
- `subheader` — the heading over a group of rows, in the theme's secondary color, which is its link color.
- `ROW_HEIGHT` — A dense `ListItemButton`: 4 px above and below a 14 px label with MUI's 4 px text margins.
- `FONT` — `Sidebar.tsx`: `fontSize: "0.875rem"`, a small icon.
- `INSET` — `ListItemButton.js`: `paddingLeft: 16`.
- `PADDING` — const
- `SUBHEADER_HEIGHT` — `ListSubheader.js`: `lineHeight: '48px'`, 14 px text.

## Depends on

- [[ListItemButton]] — uses `ListItemButton`, `new`
- [[Motion Module (mui)]] — uses `scaled`
- [[Tokens Module]] — uses `easing`
- [[Curve Module]] — uses `EASE_IN_OUT`
- [[Duration Module]] — uses `STANDARD`
- [[Themes Module]] — uses `brand`
- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `sidebar`
- [[ListItemButton]] — from `render`
- [[Menu]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]
