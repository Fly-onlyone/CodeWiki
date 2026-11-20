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
2. `AppView::edit_view` and `AppView::prepare_page` in [[codewiki-app]] call `prepare_note_with`. `prepare_note_with` calls `link_line`. `link_line` calls `encode`.
3. `AppView::details` in [[codewiki-app]] calls `outline`.

## Items

- `pub const NOTE_SCHEME: &str` — URL scheme the viewer uses for links between notes.
- `pub const DIAGRAM_SCHEME: &str` — URL scheme for diagrams rendered from Mermaid blocks, followed by an index.
- `pub const OPEN_DIAGRAM_SCHEME: &str` — URL scheme of the link around a drawn diagram, followed by the same index. Following it opens the diagram in the viewer, where it can be enlarged.
- `pub(crate) fn encode(value: &str) -> String` — internal function, returns `String`; called by `link_line`.
- `pub fn decode(value: &str) -> String` — decode `%XX` escapes produced by `encode`.
- `pub fn prepare_note(text: &str) -> String` — turn a vault note into Markdown the viewer can render: frontmatter becomes a tag line and `[[wikilinks]]` become ordinary links with the note scheme. Code spans and fenced blocks are left untouched.
- `pub enum Diagram` — what the viewer shows in place of one Mermaid block.
- `pub const DRAWING: &str` — line shown where a diagram is still being drawn.
- `pub const FIGURE_BLOCK: &str` — language of the empty code block that stands where a note's illustration is drawn. The viewer draws the illustration in place of such a block.
- `prepare_note_with` — `prepare_note` with diagram support. `diagram` receives the source of each Mermaid block and says what to show in its place. With `figure`, a `FIGURE_BLOCK` is put under the note's summary, or under its title when it has no summary: the reader meets what the note is about, then the picture of it, then the text.
- `enum Figure` — how far `prepare_note_with` is with placing the illustration.
- `fn link_line(line: &str) -> String` — replace wikilinks in one line of prose.
- `pub fn outline(text: &str) -> Vec<String>` — `##` headings of a note, for the outline pane.

## Depends on

- [[cw-client]] — uses `encode`

## Used by

- [[codewiki-app]] — from `details`, `edit_view`, `follow_link`, `page_content`

## See also

- [[_index]]
- [[codewiki-app]]
