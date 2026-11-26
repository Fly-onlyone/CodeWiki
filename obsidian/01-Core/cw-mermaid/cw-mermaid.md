---
tags: [core, cw-mermaid]
---

# cw-mermaid

> Mermaid diagram rendering for CodeWiki.

## What it does

Turns Mermaid diagram text into an image. Notes hold diagrams as Mermaid source; the app needs pictures to show. This crate is the one place that does that work, so the renderer behind it can be swapped without touching callers.

Flowcharts, sequence diagrams and state diagrams work. Any other type returns an error, and the caller shows the source text instead.

## Source

- `crates/cw-mermaid/Cargo.toml` — package manifest
- `crates/cw-mermaid/src/lib.rs` — crate root

## How it works

Three entry points, all taking the source text and returning `Result<_, String>`.

- `render_svg` returns an SVG document.
- `render_png` returns PNG bytes at one pixel per diagram unit.
- `render_png_scaled` does the same at a chosen `scale`, for a view that enlarges the picture. The app uses it to redraw a diagram sharper after zoom.

Every path first calls `lenient`. It repairs one common mistake in hand-written flowcharts: a label that is a URL path, such as `Agent[/ws/agent]`. Mermaid reads `[/` as the start of a slanted shape and rejects it. `lenient` quotes such labels. It touches only `flowchart` and `graph` sources, and leaves real slanted shapes alone.

The PNG paths then call `themed` to apply the [[Palette]], and hand the text to the `merman` renderer. If the renderer finds no diagram, the error is `no Mermaid diagram detected`.

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

## Gotchas

- The renderer holds a PNG to 4096 pixels a side. A large diagram at a big `scale` comes back smaller than asked, in the same ratio.
- Errors are plain strings, not a typed error.
- `render_svg` has no palette, so it is always light.
- Unsupported kinds, such as a pie chart, are errors. They never panic.

## See also

- [[_index]]
- [[Crate Dependency Flow]]
