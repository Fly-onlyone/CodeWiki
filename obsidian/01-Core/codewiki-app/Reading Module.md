---
tags: [core, codewiki-app, module]
---

# Reading Module

> Typography of the reading pane.

## What it does

Sets the type of the reading pane. Most of what the app shows is long Markdown, so this module gives it a bounded line length, open line spacing, a clear heading scale, and code and tables that stand apart from prose. It also decides how a link looks under the pointer.

Notes are drawn by gpui-kit's base `TextView`, whose style can color a heading. The component wrapper cannot, so the whole style is built here: headings, list markers, inline code, code blocks, table heads and quotes take the theme's hues.

## Source

- `crates/codewiki-app/src/reading.rs` — module `codewiki_app::reading`

## How it works

`frame` is the entry point for a note view. It applies `style`, a quote style with a tertiary left border and tint, a line height of 1.65, and side padding from `gutter`. `gutter` centres a column of the given measure in the pane and never goes below 24 px. `NOTE_MEASURE` (680 px) is for notes, `WIDE_MEASURE` (1040 px) for generated pages that are mostly tables.

`style` builds a `TextViewStyle`: heading sizes from `heading_scale`, colors for levels 1 to 3 from `heading_hue`, tinted code blocks and table heads, cyan inline code, and sideways scrolling for wide tables.

Link hover has two steps. `link_hover` returns a `HighlightStyle`: the link color lifted by the link knob from [[Interaction Module]] (lighter, or darker in a light theme) with a solid underline of the knob's thickness. `sync_link_hover` wraps it in `LinkHoverStyle` and stores it as a global for gpui-base's text view; the app calls it each render, and it sets the global only when the value changed. [[Doc View Module]] calls `link_hover` directly for its own linked lines.

## Items

- `pub const NOTE_MEASURE: f32` — widest a line of prose may run, in pixels. About 95 characters of the bundled interface font at its default size: longer than a book's line, because notes carry file paths and diagrams as well as prose.
- `pub const WIDE_MEASURE: f32` — measure for generated pages. They are mostly tables, which need the room.
- `const MIN_GUTTER: f32` — space kept on each side of the text however narrow the pane gets.
- `const LINE_HEIGHT: f32` — line height as a multiple of the font size.
- `const CODE_FILL: f32` — share of the primary color in a code block. A table head takes twice as much of the tertiary color.
- `const INLINE_CODE_FILL: f32` — opacity of the link color behind inline code.
- `const QUOTE_FILL: f32` — opacity of the tertiary color laid over a quote. The value was chosen by eye on 2026-11-13, when the fill was the primary color.
- `fn heading_scale(level: u8) -> f32` — heading size as a multiple of the body size, by level.
- `fn heading_hue(level: u8) -> Option<Hue>` — the hue of a heading. The top three levels each take one of the theme's text colors, which the CodeWiki themes hold to AA; deeper levels keep the body color.
- `pub fn gutter(pane_width: f32, measure: f32) -> f32` — side padding that centres a column of `measure` in a pane `pane_width` wide. It is never less than `MIN_GUTTER`.
- `fn style(cx: &App) -> TextViewStyle` — the text view's whole style: what gpui-component 0.9.1 derives from its theme (`base_text_view_style` in its `text` module), with this pane's spacing folded in as its `text::compat` folds it, and the theme's hues on top.
- `pub fn link_hover(cx: &App) -> HighlightStyle` — how a link under the pointer is drawn, over its look at rest: lighter (darker on a light theme), with a solid underline.
- `pub fn sync_link_hover(cx: &mut App)` — hand `link_hover` to gpui-base, whose text view draws the links of a note. Call while rendering; it sets the global only when it changed.
- `pub fn frame(view: TextView, pane_width: f32, measure: f32, cx: &App) -> TextView` — set `view` for reading in a pane `pane_width` wide, with lines no longer than `measure`.

## Depends on

- [[Glass Module]] — uses `tint`, `tinted_border`
- [[Interaction Module]] — uses `current`, `lift`
- [[Icons Module]] — uses `Hue`
- [[Themes Module]] — uses `brand`
- [[Zoom Module]] — uses `px`

## Used by

- [[codewiki-app]] — from `edit_view`, `margin_width`, `page_content`, `reading_measure`
- [[Doc View Module]] — from `render`

## Gotchas

- `LinkHoverStyle` exists only in the patched copy of gpui-base in `vendor/gpui-base` (see its NOTICE and the root Cargo.toml patch section for crates-io). The upstream request is longbridge/gpui-kit issue 4116. Without the patch this module does not build.
- `sync_link_hover` must run during render. If it is skipped, notes keep the plain link look.
- Deeper headings (level 4 and below) keep the body color. Only levels 1 to 3 have a hue.
- The style is whole: a field you do not set here falls back to the text view's default, not to the component theme's.
- The quote fill was picked by eye on 2026-11-13.

## See also

- [[_index]]
- [[codewiki-app]]
