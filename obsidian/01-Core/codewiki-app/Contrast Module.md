---
tags: [core, codewiki-app, module]
---

# Contrast Module

> WCAG contrast arithmetic for translucent surfaces.

## What it does

The window is glass: what sits behind a surface is not known. These functions answer "is this text still readable if the backdrop is as bad as it can be", which for any backdrop color is bounded by black and white.

## Source

- `crates/codewiki-app/src/contrast.rs` — module `codewiki_app::contrast`

## How it works

1. `debug_backdrop` in [[Glass Module]] and `parse_rgba` in [[Standard Module]] call `parse_hex`.
2. `matched_backdrop` in [[Glass Module]] and `nudge_texts` in [[Standard Module]] call `over`.
3. `Levels::compute` in [[Glass Module]] and `nudge_texts` in [[Standard Module]] call `fit_alpha`. `fit_alpha` calls `ratio`, then `worst_ratio`.
4. `readable_on` and `standard_colors` in [[Standard Module]] call `ratio`. `ratio` calls `luminance`.
5. `nudge_texts` in [[Standard Module]] calls `worst_ratio`. `worst_ratio` calls `ratio`, then `over`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub type Rgb = [f32; 3]` — an sRGB color, each channel in `0.0..=1.0`.
- `pub const BLACK: Rgb` — public constant; used by `UNKNOWN`, `matched_backdrop`, `nudge_texts`, `standard_colors`.
- `pub const WHITE: Rgb` — public constant; used by `UNKNOWN`, `matched_backdrop`, `nudge_texts`, `standard_colors`.
- `pub const AA: f32` — the contrast ratio WCAG AA asks of body text.
- `pub const PANE_FLOOR: f32` — the least opaque the reading pane may be. Every text color of the CodeWiki themes is checked at this value; at 0.85 the status colors of the light themes fall below AA.
- `pub fn parse_hex(hex: &str) -> Option<Rgb>` — parse `#RRGGBB`. An alpha pair after it is ignored.
- `pub fn over(top: Rgb, alpha: f32, under: Rgb) -> Rgb` — `top` at `alpha` painted over an opaque `under`.
- `fn luminance(color: Rgb) -> f32` — private function, returns `f32`; called by `ratio`.
- `pub fn ratio(a: Rgb, b: Rgb) -> f32` — WCAG contrast ratio between two opaque colors, from 1 to 21.
- `pub fn worst_ratio(text: Rgb, surface: Rgb, alpha: f32, backdrops: [Rgb; 2]) -> f32` — the lowest contrast `text` can have on `surface` painted at `alpha` over a backdrop that lies between the two given colors.
- `const KEEP: f32` — A theme whose text is below AA even on its opaque surface cannot be held to AA. Glass may cost such a text at most this share of what it has.
- `pub fn fit_alpha(surface: Rgb, texts: &[Rgb], floor: f32, backdrops: [Rgb; 2]) -> f32` — the lowest opacity, from `floor` up, at which every color in `texts` stays readable on `surface` over any backdrop between the two given colors. Readable means AA, or `KEEP` of the text's own contrast where that is lower. Returns 1.0 when no translucent surface manages it.

## Used by

- [[Standard Module]] — from `editor_colors`, `from_oklch`, `hex`, `nudge_texts`
- [[Glass Module]] — from `DARK_MICA_LIMIT`, `LIGHT_MICA_LIMIT`, `UNKNOWN`, `compute`
- [[Themes Module]] — from `swatches`, `to_hsla`, `to_rgb`

## See also

- [[_index]]
- [[codewiki-app]]
