---
tags: [core, cw-mermaid]
---

# cw-mermaid

> Mermaid diagram rendering for CodeWiki.

## Source

- `crates/cw-mermaid/Cargo.toml` — package manifest
- `crates/cw-mermaid/src/lib.rs` — crate root

## How it works

Builds a library.

Defined at the crate root:

- [[Palette]] — diagram palette.
- `lenient` — repair a mistake common in hand-written flowcharts: a node label that is a URL path, `Agent/ws/agent`.
- `render_png` — render Mermaid source to PNG bytes.
- `render_svg` — render Mermaid source to an SVG document.
- `themed` — apply a palette with Mermaid's own `init` directive, unless the diagram already carries one.

## Depends on

- [[Palette]] — uses `Palette`

## Used by

- [[codewiki-app]] — from `prepare_page`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
