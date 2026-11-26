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

Colors are `Rgb` arrays of three floats. `parse_hex` reads `#RRGGBB` (an alpha pair is ignored). `over` mixes a top color at an alpha over an opaque one. `ratio` gives the WCAG contrast ratio of two opaque colors, from `luminance`, between 1 and 21.

The backdrop of a glass window is unknown, so `worst_ratio` tests the surface at its alpha over both ends of a backdrop range (usually black and white) and keeps the lower contrast.

`fit_alpha` is the main entry. It starts at a floor opacity and raises it in steps of 0.02 until every text color is readable on the surface over the worst backdrop. Readable means the AA ratio of 4.5. If a text is weaker than that even on the opaque surface, the requirement drops to 90 percent of its own contrast (`KEEP`). When no opacity below 1.0 works, it returns 1.0, which means no glass.

The glass and standard-colors code use these results to decide how opaque the chrome and the reading pane must be.

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

## Gotchas

- `PANE_FLOOR` is 0.88 on purpose. At 0.85 the status colors of the light themes fall below AA.
- `parse_hex` accepts only `#` followed by 6 or 8 hex digits. Short forms such as `#fff` return `None`.
- Luminance is computed from sRGB with the WCAG formula, so the numbers match the published reference values (black on white is 21).
- The search steps up in 0.02 jumps, so the returned opacity can be up to one step above the true minimum.
- A theme with weak text gets a relaxed target, not AA. Do not read a passing result as proof of AA for every theme.

## See also

- [[_index]]
- [[codewiki-app]]
