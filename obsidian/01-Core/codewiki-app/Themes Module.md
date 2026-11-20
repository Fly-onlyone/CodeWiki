---
tags: [core, codewiki-app, module]
---

# Themes Module

> Bundled color themes and theme switching.

## Source

- `crates/codewiki-app/src/themes.rs` — module `codewiki_app::themes`

## How it works

Main items:

- `Entry` — A theme the user can choose.
- `Brand` — the three colors a theme is known by.
- `entries` — the themes the user can choose, in menu order.
- `select` — apply a theme by name and remember the choice.
- `brand` — the brand colors of the active theme.
- `glyph_tint` — `color` for an icon on the active theme's menu.
- `init` — load bundled and user themes, then apply the saved or default theme.
- `Baseline` — fonts, radius and shadow as they were before any theme was applied.

## Depends on

- [[Prefs Module]] — uses `dir`, `read`, `write`
- [[Contrast Module]] — uses `Rgb`, `parse_hex`
- [[Standard Module]] — uses `readable_on`, `standardize`
- [[Glyph]] — uses `path`

## Used by

- [[codewiki-app]] — from `main`, `open_theme_menu`, `palette_matches`, `select_theme`
- [[Reading Module]] — from `frame`, `style`
- [[Standard Module]] — from `REFERENCE`, `from_json`
- [[Backdrop Module]] — from `shapes`
- [[Doc View Module]] — from `choice_tint`
- [[List Module]] — from `subheader`
- [[ListItemButton]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]
