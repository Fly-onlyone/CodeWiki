---
tags: [core, codewiki-app, module]
---

# Reading Module

> Typography of the reading pane.

## What it does

Most of what the app shows is long Markdown, so the pane is set for reading: a bounded line length, open line spacing, headings on a clear scale, and code and tables that stand apart from the prose.

Notes are drawn with gpui-kit's base `TextView`, whose style can give a heading a color; the component wrapper only lets a heading change size. The style is therefore built here in full. Headings, list markers, inline code, code blocks, table heads and quotes take the theme's hues: a note's title and its list markers the secondary color, as links do, and second-level headings, table heads and quotes the tertiary one.

## Source

- `crates/codewiki-app/src/reading.rs` — module `codewiki_app::reading`

## How it works

1. `DocPage::render` in [[Doc View Module]] calls `gutter`.
2. `AppView::edit_view` and `AppView::page_content` in [[codewiki-app]] call `frame`. `frame` calls `brand` in [[Themes Module]], then `style`, then `px` in [[Zoom Module]], then `gutter`. `style` calls `brand` in [[Themes Module]], then `tint`, then `tinted_border` (both in [[Glass Module]]), then `heading_hue`, then `heading_scale`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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
- `pub fn frame(view: TextView, pane_width: f32, measure: f32, cx: &App) -> TextView` — set `view` for reading in a pane `pane_width` wide, with lines no longer than `measure`.

## Depends on

- [[Glass Module]] — uses `tint`, `tinted_border`
- [[Icons Module]] — uses `Hue`
- [[Themes Module]] — uses `brand`
- [[Zoom Module]] — uses `px`

## Used by

- [[codewiki-app]] — from `edit_view`, `page_content`
- [[Doc View Module]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]
