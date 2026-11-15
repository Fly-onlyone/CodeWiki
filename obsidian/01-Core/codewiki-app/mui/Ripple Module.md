---
tags: [core, codewiki-app, module]
---

# Ripple Module

> The ripple a control answers a press with.

## Source

- `crates/codewiki-app/src/mui/ripple.rs` — module `codewiki_app::mui::ripple`

## How it works

Main items:

- `Point` — type alias
- `Wave` — one ripple: when the press began and, once it has, when it ended.
- `DURATION` — `TouchRipple.js`, `DURATION`: the time a ripple takes to grow, and to fade.
- `origin` — where a ripple starts.
- `paint` — paint `waves` over a control at `bounds` whose corners have `radius`.
- `paint_pressed` — the tint a pressed control shows in place of a ripple when motion is reduced: the ripple's resting opacity over the whole control.
- `clip` — the part of `subject` inside the convex polygon `bounds` (Sutherland and Hodgman).
- `covers` — whether a disc covers the whole `width` by `height` rectangle.

## Depends on

- [[Motion Module (mui)]] — uses `scaled`
- [[Tokens Module]] — uses `easing`
- [[Curve Module]] — uses `EASE_IN_OUT`

## Used by

- [[Base Module]] — from `Touch`, `attach`, `new`, `press`

## See also

- [[_index]]
- [[codewiki-app]]
