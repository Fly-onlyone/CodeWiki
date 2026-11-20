---
tags: [core, cw-mermaid]
---

# cw-mermaid

> Mermaid diagram rendering for CodeWiki.

## What it does

All rendering goes through this crate so the renderer can be replaced in one place. Flowcharts, sequence diagrams and state diagrams are supported; any other diagram type returns an error and the caller shows the source.

## Source

- `crates/cw-mermaid/Cargo.toml` — package manifest
- `crates/cw-mermaid/src/lib.rs` — crate root

## How it works

Builds a library.

1. `AppView::draw_queued_diagrams` in [[codewiki-app]] calls `render_png`. `render_png` calls `render_png_scaled`.
2. `AppView::sharpen_diagram` in [[codewiki-app]] calls `render_png_scaled`. `render_png_scaled` calls `themed`, then `lenient`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Defined at the crate root:

- [[Palette]] — diagram palette.
- `fn themed(source: &str, palette: Palette) -> String` — apply a palette with Mermaid's own `init` directive, unless the diagram already carries one.
- `pub fn lenient(source: &str) -> String` — repair a mistake common in hand-written flowcharts: a node label that is a URL path, `Agent[/ws/agent]`. Mermaid reads `[/` as the start of a slanted shape and rejects the diagram when no `/]` or `\]` closes it. Such labels are quoted, `Agent["/ws/agent"]`; real slanted shapes and other diagram types are left alone.
- `pub fn render_svg(source: &str) -> Result<String, String>` — render Mermaid source to an SVG document.
- `pub fn render_png(source: &str, palette: Palette) -> Result<Vec<u8>, String>` — render Mermaid source to PNG bytes, one pixel per unit of the diagram.
- `pub fn render_png_scaled(source: &str, palette: Palette, scale: f32) -> Result<Vec<u8>, String>` — `render_png` with `scale` pixels per unit of the diagram, for a view that enlarges it. The renderer holds an image to 4096 pixels a side, so a large diagram comes back smaller than `scale` asks for, in its ratio.

## Depends on

- [[Palette]] — uses `Palette`

## Used by

- [[codewiki-app]] — from `draw_queued_diagrams`, `sharpen_diagram`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
