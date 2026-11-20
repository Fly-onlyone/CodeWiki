---
tags: [core, codewiki-app, module]
---

# Reading Module

> Typography of the reading pane.

## Source

- `crates/codewiki-app/src/reading.rs` — module `codewiki_app::reading`

## How it works

Main items:

- `gutter` — side padding that centres a column of `measure` in a pane `pane_width` wide.
- `frame` — set `view` for reading in a pane `pane_width` wide, with lines no longer than `measure`.
- `WIDE_MEASURE` — measure for generated pages.
- `NOTE_MEASURE` — widest a line of prose may run, in pixels.
- `MIN_GUTTER` — space kept on each side of the text however narrow the pane gets.
- `style` — the text view's whole style: what gpui-component 0.9.1 derives from its theme (`base_text_view_style` in its `text` module), with this pane's spacing folded in as its `text::compat` folds it, and the theme's hues on top.
- `heading_hue` — the hue of a heading.
- `heading_scale` — heading size as a multiple of the body size, by level.

## Depends on

- [[Glass Module]] — uses `tint`, `tinted_border`
- [[Icons Module]] — uses `Hue`
- [[Themes Module]] — uses `brand`

## Used by

- [[codewiki-app]] — from `edit_view`, `viewer`
- [[Doc View Module]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]
