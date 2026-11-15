---
tags: [core, codewiki-app, module]
---

# Doc Module

> What a generated page says, apart from how it is drawn.

## Source

- `crates/codewiki-app/src/doc.rs` — module `codewiki_app::doc`

## How it works

Main items:

- [[Doc]] — A generated page.
- [[Action]] — what following a link or pressing a row does.
- [[Span]] — A stretch of text within a line.
- [[Block]] — A part of a page.
- `Row` — A row that can be pressed.
- `Cell` — what a table cell holds.
- `Column` — A column of a table.
- `Table` — A table: columns, and rows of one cell for each.

## Depends on

- [[Action]] — uses `Action`
- [[Span]] — uses `Span`
- [[Glyph]] — uses `Glyph`
- [[Tone]] — uses `Tone`

## Used by

- [[Pages Module]] — from `contracts_page`, `coupling_page`, `coverage_page`, `dead_code_page`
- [[Live Module]] — from `file_page`, `file_row`, `home_page`, `hotspots_page`
- [[Doc View Module]] — from `choices_row`, `draw_cell`, `draw_row`, `draw_table`
- [[Doc]] — from `tables`, `text`, `words`
- [[Illustration Module]] — from `render`, `table`
- [[Block]] — from `Block`

## See also

- [[_index]]
- [[codewiki-app]]
