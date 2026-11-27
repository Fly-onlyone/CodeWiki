---
tags: [core, codewiki-app, module]
---

# Markdown Module

> Text preparation for the note viewer.

## What it does

A vault note is Obsidian Markdown: it has frontmatter, `[[wikilinks]]` and Mermaid blocks. The note viewer cannot show those as they are. This module rewrites a note's text into plain Markdown the viewer can draw, and makes the small link scheme the viewer uses to move between notes and open diagrams. It has no UI types, so unit tests cover it.

## Source

- `crates/codewiki-app/src/markdown.rs` — module `codewiki_app::markdown`

## How it works

`prepare_note_with` walks the note line by line, and builds new text.

- Frontmatter is cut off. Its `tags:` line becomes a row of inline code right under the `# ` title.
- Each Mermaid block goes to a callback that decides what to show: a linked image (`Diagram::Image`), a line saying `*Drawing diagram…*`, or the block as code. The image link points at `codewiki-diagram://<index>` and is wrapped in a link to `codewiki://diagram/<index>`, so a click opens the enlarged viewer.
- `link_line` turns `[[Target|Label]]` in prose into an ordinary link to `codewiki://note/<encoded target>`. The part after `#` is dropped, and a trailing `/` on the label is trimmed.
- Under each `## ` heading named in `figures`, it puts a code block in the language `codewiki-figure` that holds the picture's number. The viewer replaces it with the drawn picture, read back by `figure_index`.

`follow_link` calls `decode` to read the note name from a clicked link. `outline` lists the `##` headings for the outline pane.

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

## Depends on

- [[cw-client]] — uses `encode`

## Used by

- [[codewiki-app]] — from `edit_view`, `follow_link`, `page_content`, `prepare_page`

## Gotchas

- Wikilinks inside inline code or fenced blocks are left alone, so `[[not a link]]` in backticks stays as text.
- A `## ` line inside a code block is code. It does not get a picture and is not in the outline.
- A heading that appears twice gets its picture only once, under the first.
- A heading in `figures` that the note lacks gets no picture.
- `decode` only reads `%XX` escapes. Text from `encode` round-trips, and a bad escape is kept as it is.
- A Mermaid block must close with a line that starts with three backticks. An unclosed block swallows the rest of the note.
- A leading byte order mark is removed before frontmatter is read.

## See also

- [[_index]]
- [[codewiki-app]]
