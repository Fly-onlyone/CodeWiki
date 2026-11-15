---
tags: [core, codewiki-app, module]
---

# Icons Module

> Icons, and the asset source that serves them.

## Source

- `crates/codewiki-app/src/icons.rs` — module `codewiki_app::icons`

## How it works

Main items:

- [[Glyph]] — an icon of the app: the path its SVG is served under.
- `Hue` — the theme color an icon is drawn in, so that the chrome carries the theme's palette and each kind of thing keeps the same color.
- `AppAssets` — the app's asset source: its own icons first, then gpui-kit's defaults, which the kit's components need.
- `hue` — the hue of `icon` in the sidebar and the toolbar, or `None` for an icon that stays neutral, such as back or close.
- `LOGO` — the app's own logo, the same art as its Windows icon, served under this path.
- `tint` — `hue` in the active theme.
- `for_command` — the icon shown beside a palette entry.
- `for_theme` — the icon that stands for the theme `name` in the theme menu.

## Depends on

- [[Palette Module]] — uses `Command`
- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `main`, `palette_view`, `sidebar`, `tab_bar`
- [[Doc View Module]] — from `draw`
- [[Reading Module]] — from `heading_hue`

## See also

- [[_index]]
- [[codewiki-app]]
