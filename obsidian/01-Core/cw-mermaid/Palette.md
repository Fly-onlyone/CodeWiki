---
tags: [core, cw-mermaid]
---

# Palette

> Diagram palette.

## What it does

Says which colour theme a diagram is drawn in: `Light` or `Dark`. The caller picks the one that matches the surface the picture will sit on, so a diagram does not show dark text on a dark page.

## Source

- `crates/cw-mermaid/src/lib.rs` — definition of `Palette`

## How it works

`render_png` and `render_png_scaled` take a `Palette`. Before drawing, `themed` applies it.

For `Light`, the source is left as it is, because Mermaid draws light by default. For `Dark`, `themed` puts Mermaid's own `init` directive with the `dark` theme in front of the source.

`render_svg` takes no palette. It always draws the default light theme.

## Items

Declared as `pub enum Palette`.

## Used by

- [[cw-mermaid]] — from `render_png`, `render_png_scaled`, `themed`
- [[Diagrams Module]] — from `Key`, `lookup`
- [[codewiki-app]] — from `AppView`

## Gotchas

- If the source already starts with a `%%{` directive, `themed` leaves it alone, even for `Dark`. The diagram's own theme wins.
- Only the first non-blank text is checked. A directive placed later in the source does not stop the dark one from being added.
- The palette only changes PNG output. SVG output has no palette.

## See also

- [[_index]]
- [[cw-mermaid]]
