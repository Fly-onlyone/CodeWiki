---
tags: [core, codewiki-app, module]
---

# Icons Module

> Icons, and the asset source that serves them.

## What it does

The app draws Material Icons, as Material UI apps do. Each is an SVG file in `icons/`, built from the path data of `@mui/icons-material`, and is embedded in the binary here.

Every icon the app draws is a constant here. gpui-kit's own components draw a few Lucide icons of their own; those come from the kit's assets.

## Source

- `crates/codewiki-app/src/icons.rs` — module `codewiki_app::icons`

## How it works

1. `theme_menu_item` in [[codewiki-app]] and `appearance` in [[Settings Pages Module]] call `for_theme`.
2. `AppView::sidebar` and `AppView::tab_bar` in [[codewiki-app]], and 4 more, call `tint`. `tint` calls `hue`.
3. `AppView::palette_view` in [[codewiki-app]] calls `for_command`.

## Items

- [[Glyph]] — an icon of the app: the path its SVG is served under.
- `const FOLDER_PATH: &str` — where the app's icons are served, apart from the kit's under `icons/`.
- `app_icons` — declare the app's icons: embed each file, and name it as a constant.
- `pub fn for_theme(name: &str) -> Glyph` — the icon that stands for the theme `name` in the theme menu. A light theme shares the icon of its dark twin; a theme the app does not know, such as one of the user's own, takes the palette.
- `pub const LOGO: &str` — the app's own logo, the same art as its Windows icon, served under this path. Draw it with `img(icons::LOGO)`.
- `pub const LOGO_BYTES: &[u8]` — the logo's PNG file, which the header of a Windows notification shows too (`identity.rs`).
- `pub fn for_command(command: &Command) -> Glyph` — the icon shown beside a palette entry.
- `pub enum Hue` — the theme color an icon is drawn in, so that the chrome carries the theme's palette and each kind of thing keeps the same color.
- `pub fn hue(icon: Glyph) -> Option<Hue>` — the hue of `icon` in the sidebar and the toolbar, or `None` for an icon that stays neutral, such as back or close.
- `pub fn tint(icon: Glyph, cx: &App) -> Option<Hsla>` — `hue` in the active theme.
- `pub struct AppAssets` — the app's asset source: its own icons first, then gpui-kit's defaults, which the kit's components need.

## Depends on

- [[Palette Module]] — uses `Command`
- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `main`, `palette_view`, `settings_entry`, `settings_frame`
- [[Doc View Module]] — from `choice_tint`, `draw`
- [[Instance Module]] — from `listen`
- [[Snackbar Module]] — from `snackbar`
- [[Reading Module]] — from `heading_hue`
- [[Settings Pages Module]] — from `appearance`
- [[Win32 Module]] — from `tray_procedure`

## See also

- [[_index]]
- [[codewiki-app]]
