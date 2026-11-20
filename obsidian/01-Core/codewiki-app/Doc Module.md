---
tags: [core, codewiki-app, module]
---

# Doc Module

> What a generated page says, apart from how it is drawn.

## Source

- `crates/codewiki-app/src/doc.rs` — module `codewiki_app::doc`

## How it works

Main items:

- [[Action]] — what following a link or pressing a row does.
- [[Doc]] — A generated page.
- [[Span]] — A stretch of text within a line.
- [[Block]] — A part of a page.
- `Row` — A row that can be pressed.
- `Cell` — what a table cell holds.
- `Column` — A column of a table.
- `text` — function

## Depends on

- [[Settings Module]] — uses `Note`
- [[Action]] — uses `Action`
- [[Span]] — uses `Span`
- [[Glyph]] — uses `Glyph`
- [[Tone]] — uses `Tone`

## Used by

- [[Settings Pages Module]] — from `ai_model`, `appearance`, `backend`, `built_in`
- [[Live Module]] — from `file_page`, `file_row`, `home_page`, `hotspots_page`
- [[Doc View Module]] — from `SelectRequest`, `choice_tint`, `choices_row`, `draw_cell`
- [[Pages Module]] — from `contracts_page`, `coupling_page`, `coverage_page`, `dead_code_page`
- [[Doc]] — from `tables`, `text`, `words`
- [[Illustration Module]] — from `render`, `table`
- [[Settings View Module]] — from `OpenMenu`, `lists_themes`
- [[Block]] — from `Block`

## See also

- [[_index]]
- [[codewiki-app]]
