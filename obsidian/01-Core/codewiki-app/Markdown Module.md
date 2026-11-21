---
tags: [core, codewiki-app, module]
---

# Markdown Module

> Text preparation for the note viewer.

## What it does

No UI types, so it is unit-tested.

## Source

- `crates/codewiki-app/src/markdown.rs` — module `codewiki_app::markdown`

## How it works

1. `AppView::follow_link` in [[codewiki-app]] calls `decode`.
2. `AppView::page_content` in [[codewiki-app]] calls `figure_index`.
3. `AppView::edit_view` and `AppView::prepare_page` in [[codewiki-app]] call `prepare_note_with`. `prepare_note_with` calls `link_line`. `link_line` calls `encode`.
4. `AppView::details` in [[codewiki-app]] calls `outline`.

## Items

- `pub const NOTE_SCHEME: &str` — URL scheme the viewer uses for links between notes.
- `pub const DIAGRAM_SCHEME: &str` — URL scheme for diagrams rendered from Mermaid blocks, followed by an index.
- `pub const OPEN_DIAGRAM_SCHEME: &str` — URL scheme of the link around a drawn diagram, followed by the same index. Following it opens the diagram in the viewer, where it can be enlarged.
- `pub(crate) fn encode(value: &str) -> String` — internal function, returns `String`; called by `link_line`.
- `pub fn decode(value: &str) -> String` — decode `%XX` escapes produced by `encode`.
- `pub fn prepare_note(text: &str) -> String` — turn a vault note into Markdown the viewer can render: frontmatter becomes a tag line and `[[wikilinks]]` become ordinary links with the note scheme. Code spans and fenced blocks are left untouched.
- `pub enum Diagram` — what the viewer shows in place of one Mermaid block.
- `pub const DRAWING: &str` — line shown where a diagram is still being drawn.
- `pub const FIGURE_BLOCK: &str` — language of the code block that stands where one of a note's pictures is drawn. The block holds the picture's number among the note's pictures, and the viewer draws that picture in its place.
- `pub fn figure_index(block: &str) -> Option<usize>` — the number of the picture a `FIGURE_BLOCK` with this text stands for.
- `prepare_note_with` — `prepare_note` with diagram support. `diagram` receives the source of each Mermaid block and says what to show in its place. `figures` are the headings of the sections that have a picture. A `FIGURE_BLOCK` is put directly under each such heading: a picture stands inside the section it explains, and the reader meets it before the text it makes easier.
- `fn link_line(line: &str) -> String` — replace wikilinks in one line of prose.
- `pub fn outline(text: &str) -> Vec<String>` — `##` headings of a note, for the outline pane.

## Depends on

- [[cw-client]] — uses `encode`

## Used by

- [[codewiki-app]] — from `details`, `edit_view`, `follow_link`, `page_content`

## See also

- [[_index]]
- [[codewiki-app]]
