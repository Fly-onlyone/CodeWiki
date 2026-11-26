---
tags: [core, codewiki-app, module]
---

# Themes Module

> Bundled color themes and theme switching.

## What it does

This module owns the color themes: it loads the bundled ones, loads the user's own, applies the chosen one, and remembers the choice. The bundled set is CodeWiki's own family (four hues, each dark and light, with bundled fonts and a tighter radius) plus Dracula, Nord, Gruvbox and Rosé Pine, written from their authors' published palettes. Every file names the palette author. User themes dropped in the `themes` folder of the app's data folder load too, and [[Standard Module]] redraws them on the CodeWiki set's levels. It also defines `Brand`, the primary, secondary and tertiary colors that the rest of the UI asks for.

## Source

- `crates/codewiki-app/src/themes.rs` — module `codewiki_app::themes`

## How it works

`init` runs at start. It first captures a `Baseline` of fonts, radius and shadow before any theme is applied. Then it loads the bundled JSON files and every JSON file in the user folder into the kit's registry; a broken file is skipped with a message. It builds the `Catalog` of themes on offer (the kit's two own themes are hidden), sorted dark first, then light, with bundled themes before the user's own. Each entry carries four swatches for the menu, taken from the standardized theme. Finally `resolve` picks the theme from the saved setting and `apply` applies it.

`apply` standardizes the config, restores the baseline, applies the config and rebases zoom so sizes are those at 100% before scaling. `select` applies by name and saves the choice through [[Prefs Module]].

`Brand::of` maps primary to the theme's primary, secondary to its link color, and tertiary to its sidebar primary. The kit draws nothing in the third, so the app uses it for what should stand apart.

`glyph_tint` lightens or darkens a theme's menu icon, in its own hue, until it has 3:1 contrast on the popover surface.

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

## Gotchas

- Without the baseline reset, a theme that names no fonts or radius would inherit those of the previously active theme.
- The baseline must be captured before zoom starts, or it would hold scaled sizes.
- A saved theme that no longer exists falls back to the default of its mode, judged only by whether its name ends with Light, and the setting is rewritten.
- Names matter: the bundled list decides which themes skip standardizing. A user theme with the same name as a bundled one is treated as bundled.
- Theme files are embedded in the program at build time; editing them needs a rebuild.

## See also

- [[_index]]
- [[codewiki-app]]
