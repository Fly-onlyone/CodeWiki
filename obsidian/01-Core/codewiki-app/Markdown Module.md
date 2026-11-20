---
tags: [core, codewiki-app, module]
---

# Markdown Module

> Text preparation for the note viewer.

## Source

- `crates/codewiki-app/src/markdown.rs` — module `codewiki_app::markdown`

## How it works

No UI types, so it is unit-tested.

Main items:

- `prepare_note_with` — `prepare_note` with diagram support.
- `NOTE_SCHEME` — URL scheme the viewer uses for links between notes.
- `DIAGRAM_SCHEME` — URL scheme for diagrams rendered from Mermaid blocks, followed by an index.
- `DRAWING` — line shown where a diagram is still being drawn.
- `Diagram` — what the viewer shows in place of one Mermaid block.
- `decode` — decode `%XX` escapes produced by `encode`.
- `outline` — `##` headings of a note, for the outline pane.
- `prepare_note` — turn a vault note into Markdown the viewer can render: frontmatter becomes a tag line and `[[wikilinks]]` become ordinary links with the note scheme.

## Depends on

- [[cw-client]] — uses `encode`

## Used by

- [[codewiki-app]] — from `details`, `edit_view`, `follow_link`, `prepare_page`

## See also

- [[_index]]
- [[codewiki-app]]
