---
tags: [core, codewiki-app, module]
---

# Standard Module

> Standard levels for a theme the app did not ship.

## Source

- `crates/codewiki-app/src/standard.rs` — module `codewiki_app::standard`

## How it works

Main items:

- `standardize` — `config` on the CodeWiki set's levels.
- `readable_on` — `color` moved in lightness, away from `surface`, until the two are `target` apart in contrast.
- `Reference` — struct
- `linear_from_oklch` — OKLCH to linear sRGB, which may fall outside 0–1.
- `Level` — one color key's level in the CodeWiki set: mean OKLCH lightness and chroma, mean hue (used when a theme has no hue of its own for the key), and mean alpha.
- `hex` — function
- `to_linear` — function
- `Role` — what a color key is for, which decides the hue it is drawn in.

## Depends on

- [[Contrast Module]] — uses `AA`, `BLACK`, `PANE_FLOOR`, `Rgb`
- [[Glass Module]] — uses `CHROME_MATCHED_FLOOR`, `CHROME_TINT`, `PANE_TINT`, `matched_backdrop`
- [[Themes Module]] — uses `BUNDLED_NAMES`, `CODEWIKI`

## Used by

- [[Themes Module]] — from `apply`, `glyph_tint`, `init`

## See also

- [[_index]]
- [[codewiki-app]]
