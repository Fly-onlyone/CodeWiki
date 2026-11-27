---
tags: [core, codewiki-app, module]
---

# Icons Module

> Icons, and the asset source that serves them.

## What it does

This module holds every icon the app draws, and serves them to GPUI. The icons are Material Icons: SVG files built from the path data of `@mui/icons-material`, compiled into the program. Each one is a named constant of type [[Glyph]]. The module also holds the app logo, and maps themes and palette commands to icons. It also decides which icons are drawn in a theme color.

## Source

- `crates/codewiki-app/src/icons.rs` — module `codewiki_app::icons`

## How it works

The `app_icons!` macro turns each line into a `Glyph` constant and adds the SVG bytes to one `EMBEDDED` list. Some constants serve one place only: `HISTORY` is the toolbar's Open recent menu, `AGENT` the agent view of a note, `WAITING` the Waiting notes page, and `CONNECTED` the toast that says the backend answers again. `AppAssets` is the asset source the app registers at startup. Its `load` method first answers the logo path with the embedded PNG, then looks the path up in `EMBEDDED`, and otherwise passes the request to gpui-kit's own `Assets` (the kit draws a few Lucide icons itself). `list` merges both sources and drops duplicates.

Three small functions choose icons. `for_theme` maps a theme name to its icon, and a light theme shares its dark twin's icon (it strips the " Light" suffix). An unknown theme gets the palette icon. `for_command` gives the icon for each palette entry; a command with a settings category takes the category's own icon. `hue` says which theme color an icon wears in the sidebar and toolbar, and `tint` turns that hue into a color in the active theme; icons like back and close stay neutral.

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

- [[codewiki-app]] — from `main`, `palette_view`, `recent_menu_view`, `settings_entry`
- [[Doc View Module]] — from `choice_tint`, `draw`
- [[Instance Module]] — from `listen`
- [[Snackbar Module]] — from `snackbar`
- [[Reading Module]] — from `heading_hue`
- [[Settings Pages Module]] — from `appearance`
- [[Win32 Module]] — from `tray_procedure`

## Gotchas

- `for_command` matches every `Command` with no fallback arm. A new palette command will not compile until it gets an icon.
- `for_theme` matches theme names as strings. A renamed bundled theme silently falls back to the palette icon, until the theme-icon test catches it.
- An icon missing from `hue` has no color. Add an arm if a new icon should follow the theme.
- `LOGO_BYTES` is also what the Windows notification header shows. [[Identity Module]] writes it to disk, so a change to the logo file changes both.
- Tests require each SVG to start with `<svg`, use a 24 unit view box and have a path. A file from another icon set may fail them.
- `list` must not return a path twice. A test checks it.

## See also

- [[_index]]
- [[codewiki-app]]
