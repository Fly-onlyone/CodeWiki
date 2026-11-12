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

- `FILE_SCHEME` — URL scheme for links to live file pages.
- `NOTE_SCHEME` — URL scheme the viewer uses for links between notes.
- `prepare_note_with` — `prepare_note` with diagram support.
- `THEME_SCHEME` — URL scheme for links that apply a theme.
- `SYMBOL_SCHEME` — URL scheme for links to symbol detail pages.
- `DIAGRAM_SCHEME` — URL scheme for diagrams rendered from Mermaid blocks, followed by an index.
- `proposal_page` — render one proposal for review.
- `hotspots_page` — render the hotspot ranking as a table.

## Depends on

- [[cw-client]] — uses `encode`

## Used by

- [[codewiki-app]] — from `details`, `edit_view`, `follow_link`, `open_file`
- [[Pages Module]] — from `coupling_page`, `coverage_page`, `dead_code_page`, `decisions_page`

## See also

- [[_index]]
- [[codewiki-app]]
