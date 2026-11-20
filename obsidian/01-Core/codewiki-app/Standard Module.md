---
tags: [core, codewiki-app, module]
---

# Standard Module

> Standard levels for a theme the app did not ship.

## What it does

Themes come from many authors and sit at very different levels: some are near black with a dark accent, some sit on a light gray with a pastel one. Under the app's glass that spread shows as themes that are far too dark or too bright, with text that cannot be read. A bundled theme was written for this app and is drawn as its file has it. Any other theme, which is one of the user's own, is redrawn on the CodeWiki set's levels: each color role takes the lightness and saturation the CodeWiki themes of the same mode give it, in the theme's own hue. Code syntax colors stay as the author made them.

A bundled theme has three brand hues: primary, secondary and tertiary (`themes::Brand`). A foreign theme seldom names more than one, so the other two are found among the colors it does name, or set beside the primary hue on the color wheel.

The levels are read from `codewiki.json` itself, so they follow any change to the CodeWiki set. Text colors are then nudged until they keep the contrast `glass` and the theme tests ask of the CodeWiki themes.

## Source

- `crates/codewiki-app/src/standard.rs` — module `codewiki_app::standard`

## How it works

1. `apply` and `init` in [[Themes Module]] call `standardize`. `standardize` calls `standard_colors`, then `hex`, then `editor_colors`. `standard_colors` calls `to_oklch`, then `brand_hues`, then `from_oklch`, then `nudge_texts`, then `ratio` in [[Contrast Module]]. `editor_colors` calls `hex`.
2. `glyph_tint` in [[Themes Module]] calls `readable_on`. `readable_on` calls `to_oklch`, then `ratio` in [[Contrast Module]], then `from_oklch`. `to_oklch` calls `to_linear`. `from_oklch` calls `max_chroma`, then `linear_from_oklch`, then `to_gamma`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `enum Role` — what a color key is for, which decides the hue it is drawn in.
- `pub(crate) const MIN_CHROMA: f64` — A hue is only taken from a color with at least this much chroma; below it the color reads as gray and its hue is noise.
- `pub(crate) const SECONDARY_APART: (f64, f64)` — how far, in degrees of hue, a secondary color sits from the primary one: near enough to go with it, far enough to be told from it.
- `pub(crate) const TERTIARY_APART: f64` — the least a tertiary color sits from the primary one.
- `pub(crate) const BRAND_APART: f64` — the least the secondary and tertiary colors sit from each other.
- `struct Level` — one color key's level in the CodeWiki set: mean OKLCH lightness and chroma, mean hue (used when a theme has no hue of its own for the key), and mean alpha.
- `pub fn standardize(config: &ThemeConfig) -> ThemeConfig` — `config` on the CodeWiki set's levels. The bundled themes, and any theme whose colors cannot be read, are returned unchanged.
- `pub(crate) fn hue_apart(a: f64, b: f64) -> f64` — the distance between two hues, in degrees, the short way round.
- `fn brand_hues(own: impl Fn(&str) -> Option<[f64; 3]>) -> [Option<f64>; 3]` — the hues of a theme's primary, secondary and tertiary colors, given the chromatic colors it names. All are `None` for a theme with no hue at all, which stays gray. The secondary hue is the theme's link, when that is not its primary color again; else whichever spare hue sits nearest to `SECONDARY_AIM` from the primary; else that aim itself. The tertiary hue is the theme's own, when it is far enough away; else the spare hue farthest from the other two; else the one of `TERTIARY_AIMS` farthest from the other two.
- `standard_colors` — the standard color of every role key, from the theme's own `colors`.
- `fn nudge_texts(out: &mut HashMap<&'static str, (Rgb, f32)>, dark: bool)` — move each text and icon color's lightness, away from the page, until it reads on every surface it is drawn on.
- `pub fn readable_on(color: Rgb, surface: Rgb, dark: bool, target: f32) -> Rgb` — `color` moved in lightness, away from `surface`, until the two are `target` apart in contrast. Its hue and chroma are kept as far as sRGB allows. On a dark surface the color gets lighter, on a light one darker.
- `fn editor_colors(config: &ThemeConfig, new: &HashMap<&'static str, (Rgb, f32)>) -> Option<Value>` — the theme's highlight section with its editor colors set from the standard ones, or `None` when the theme has no highlight section.
- `fn hex(rgb: Rgb, alpha: f32) -> String` — private function, returns `String`; called by `editor_colors`, `standardize`.
- `fn to_linear(c: f64) -> f64` — private function, returns `f64`; called by `to_oklch`.
- `fn to_gamma(c: f64) -> f64` — private function, returns `f64`; called by `from_oklch`.
- `pub(crate) fn to_oklch(rgb: Rgb) -> [f64; 3]` — sRGB to OKLCH: lightness 0–1, chroma, hue in degrees. Björn Ottosson's OKLab matrices.
- `fn linear_from_oklch(l: f64, c: f64, h: f64) -> [f64; 3]` — OKLCH to linear sRGB, which may fall outside 0–1.
- `fn max_chroma(l: f64, h: f64) -> f64` — the most chroma a color of lightness `l` and hue `h` can have in sRGB.
- `fn from_oklch(l: f64, c: f64, h: f64, gamut: f64) -> Rgb` — the sRGB color at OKLCH `l`, `c`, `h`, with the chroma held to `gamut` of what sRGB allows there. Channels are rounded to 8 bits, so contrast is checked on exactly the color that is written out.

And 15 more items.

## Depends on

- [[Contrast Module]] — uses `AA`, `BLACK`, `PANE_FLOOR`, `Rgb`
- [[Glass Module]] — uses `CHROME_MATCHED_FLOOR`, `CHROME_TINT`, `PANE_TINT`, `matched_backdrop`
- [[Themes Module]] — uses `BUNDLED_NAMES`, `CODEWIKI`

## Used by

- [[Themes Module]] — from `apply`, `glyph_tint`, `init`

## See also

- [[_index]]
- [[codewiki-app]]
