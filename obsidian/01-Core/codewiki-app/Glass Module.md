---
tags: [core, codewiki-app, module]
---

# Glass Module

> Glass: how see-through each surface is.

## Source

- `crates/codewiki-app/src/glass.rs` — module `codewiki_app::glass`

## How it works

Main items:

- `Levels` — opacity of the two kinds of surface, from 0 (clear) to 1 (solid).
- `tint` — `surface` with `share` of `primary` mixed in.
- `tinted_border` — the theme's border, pulled toward its primary color.
- `hairline` — A one-pixel strip across the top of a surface that starts in the primary color and fades out to the right.
- `tinted` — A chrome surface: `surface` tinted toward the active theme's primary color.
- `PANE_TINT` — share of the primary color mixed into the reading pane, as much as the chrome so that the page and its frame read as one surface.
- `tinted_pane` — the reading pane's surface: `surface` tinted toward the active theme's primary color.
- `debug_backdrop` — an opaque color to paint under every surface instead of the real backdrop, from `CODEWIKI_BACKDROP=RRGGBB`.

## Depends on

- [[Contrast Module]] — uses `BLACK`, `PANE_FLOOR`, `Rgb`, `WHITE`
- [[Backdrop Module]] — uses `STRENGTH`

## Used by

- [[codewiki-app]] — from `AppView`, `body`, `details`, `new`
- [[Surfaces Module]] — from `card`, `card_fill`, `card_piece`, `paper`
- [[Backdrop Module]] — from `new`
- [[Reading Module]] — from `style`
- [[Standard Module]] — from `nudge_texts`

## See also

- [[_index]]
- [[codewiki-app]]
