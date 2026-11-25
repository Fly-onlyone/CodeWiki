---
tags: [core, codewiki-app, module]
---

# Themes Module

> Bundled color themes and theme switching.

## What it does

`codewiki.json` is CodeWiki's own family: four hues, each dark and light, with the bundled fonts and a tighter radius. Dracula, Nord, Gruvbox and Rosé Pine were written for CodeWiki from the palettes their authors publish; every file carries the palette author's name and URL. Their backgrounds, surfaces, borders and fills are the published values. A color drawn as text or as an icon keeps its published hue and is as much lighter as it takes to be read on the app's glass.

User themes dropped into `%APPDATA%\CodeWiki\themes` are loaded as well, and redrawn on the CodeWiki set's levels (`standard.rs`).

## Source

- `crates/codewiki-app/src/themes.rs` — module `codewiki_app::themes`

## How it works

1. `Shell::shapes` and `Shell::sky` in [[Backdrop Module]], and 1 more, call `Brand::of`.
2. `subheader` in [[List Module]] and `frame` in [[Reading Module]], and 1 more, call `brand`. `brand` calls `Brand::of`.
3. `theme_menu_item` in [[codewiki-app]] and `choice_tint` in [[Doc View Module]] call `glyph_tint`. `glyph_tint` calls `to_hsla`, then `readable_on` in [[Standard Module]], then `to_rgb`.
4. `AppView::open_theme_menu` and `AppView::palette_matches` in [[codewiki-app]], and 4 more, call `entries`.
5. `main` in [[codewiki-app]] calls `init`. `init` calls `Baseline::capture`, then `user_theme_dir`, then `catalog`, then `swatches`, then `standardize` in [[Standard Module]], then `resolve`, and 3 more. `user_theme_dir` calls `dir` in [[Prefs Module]].

1 more entry point: `select`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const SETTING: &str` — name of the setting that remembers the chosen theme.
- `const DEFAULT_THEME: &str` — theme applied on first start.
- `pub(crate) const BUNDLED_NAMES: [&str; 12]` — the bundled themes, in the order the theme menu lists them: the dark ones, then the light ones.
- `pub(crate) const CODEWIKI: &str` — the CodeWiki family. Kept apart from `BUNDLED` so tests can check it alone.
- `pub(crate) const BUNDLED: [&str; 5]` — internal constant; used by `init`.
- `pub struct Entry` — A theme the user can choose.
- `pub struct Brand` — the three colors a theme is known by. Each has its own work, so that a theme shows more than one hue:
  - `primary` fills what the user acts on and what is chosen: buttons, the active tab, the selection, the focus ring.
  - `secondary` is the color of links and of what is drawn as one: section labels, a note's title, list markers.
  - `tertiary` marks what stands apart from both: the bar of the selected row, kind labels, quotes, table heads and second-level headings.
  A theme file carries them in keys the kit already reads: `primary.background`, `link` and `sidebar.primary.background`. The kit draws nothing of its own in the third.
- `pub fn brand(cx: &App) -> Brand` — the brand colors of the active theme.
- `struct Catalog` — the themes on offer, built once at start.
- `fn user_theme_dir() -> Option<std::path::PathBuf>` — private function, returns `Option<std::path::PathBuf>`; called by `init`.
- `struct Baseline` — fonts, radius and shadow as they were before any theme was applied. A theme file sets these only when it names them, and the values then stay for every later theme. Without a reset, a theme that names none of them would inherit the fonts and radius of whichever theme was active before. Captured before `zoom` starts, so the sizes are those at 100%.
- `pub fn init(cx: &mut App)` — load bundled and user themes, then apply the saved or default theme.
- `fn catalog(loaded: impl Iterator<Item = Entry>) -> Vec<Entry>` — the themes on offer, in menu order: the dark ones, then the light ones. In each mode the bundled themes come first, in `BUNDLED_NAMES` order, then the user's own by name.
- `fn to_hsla(rgb: Rgb) -> Hsla` — private function, returns `Hsla`; called by `glyph_tint`, `swatches`.
- `fn to_rgb(color: Hsla) -> Rgb` — private function, returns `Rgb`; called by `glyph_tint`.
- `fn swatches(config: &ThemeConfig) -> [Hsla; 4]` — the samples of a theme for the menu. A color the theme does not set is drawn as the kit would draw it: a link or a sidebar accent falls back to the primary color.
- `pub fn glyph_tint(color: Hsla, cx: &App) -> Hsla` — `color` for an icon on the active theme's menu. The icon of a theme in the menu is drawn in that theme's primary color, which was chosen for its own surfaces, not for these; it is lightened or darkened here, in its own hue, until it stands out.
- `fn resolve(saved: Option<&str>, entries: &[Entry]) -> (String, bool)` — the theme to start with, and whether the saved name must be replaced. A saved theme that is no longer on offer gives way to the default of its mode, as far as its name tells the mode.
- `pub fn entries(cx: &App) -> &[Entry]` — the themes the user can choose, in menu order.
- `pub fn select(name: &str, cx: &mut App) -> bool` — apply a theme by name and remember the choice. Returns false when no theme has that name.

And 3 more items.

## Depends on

- [[Prefs Module]] — uses `dir`, `read`, `write`
- [[Contrast Module]] — uses `Rgb`, `parse_hex`
- [[Standard Module]] — uses `readable_on`, `standardize`
- [[Zoom Module]] — uses `rebase_theme`
- [[Glyph]] — uses `path`

## Used by

- [[codewiki-app]] — from `main`, `open_theme_menu`, `palette_matches`, `select_theme`
- [[Backdrop Module]] — from `shapes`, `sky`
- [[List Module]] — from `render`, `subheader`
- [[Reading Module]] — from `frame`, `style`
- [[Standard Module]] — from `REFERENCE`, `from_json`
- [[Doc View Module]] — from `choice_tint`

## See also

- [[_index]]
- [[codewiki-app]]
