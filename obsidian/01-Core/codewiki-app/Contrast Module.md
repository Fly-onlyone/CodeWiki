---
tags: [core, codewiki-app, module]
---

# Contrast Module

> WCAG contrast arithmetic for translucent surfaces.

## Source

- `crates/codewiki-app/src/contrast.rs` — module `codewiki_app::contrast`

## How it works

Main items:

- `Rgb` — an sRGB color, each channel in `0.0..=1.0`.
- `parse_hex` — parse `#RRGGBB`.
- `fit_alpha` — the lowest opacity, from `floor` up, at which every color in `texts` stays readable on `surface` over any backdrop between the two given colors.
- `ratio` — WCAG contrast ratio between two opaque colors, from 1 to 21.
- `over` — `top` at `alpha` painted over an opaque `under`.
- `BLACK` — const
- `WHITE` — const
- `worst_ratio` — the lowest contrast `text` can have on `surface` painted at `alpha` over a backdrop that lies between the two given colors.

## Used by

- [[Standard Module]] — from `editor_colors`, `from_oklch`, `hex`, `nudge_texts`
- [[Glass Module]] — from `DARK_MICA_LIMIT`, `LIGHT_MICA_LIMIT`, `UNKNOWN`, `compute`
- [[Themes Module]] — from `swatches`, `to_hsla`, `to_rgb`

## See also

- [[_index]]
- [[codewiki-app]]
