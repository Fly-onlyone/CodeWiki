---
tags: [core, codewiki-app, module]
---

# Doc Module

> What a generated page says, apart from how it is drawn.

## What it does

A page function turns backend data into a `Doc`: a title, and blocks such as tables, cards and rows. `doc_view` draws a `Doc` with the `mui` components. Keeping the two apart lets a test read a page without a window.

## Source

- `crates/codewiki-app/src/doc.rs` — module `codewiki_app::doc`

## How it works

1. `home_page` and `offline_page` in [[Live Module]], and 16 more, call `text`.
2. `file_page` and `offline_page` in [[Live Module]], and 14 more, call `code`.
3. `file_page` and `hotspots_page` in [[Live Module]], and 13 more, call `Column::new`.
4. `file_page` and `hotspots_page` in [[Live Module]], and 11 more, call `Cell::text`. `Cell::text` calls `text`.
5. `file_page` and `hotspots_page` in [[Live Module]], and 13 more, call `Table::new`.

12 more entry points: `strong`, `file`, `Stat::new`, `Column::numeric`, `Cell::chip`, `Press::new`, and 6 more.

## Items

- [[Action]] — what following a link or pressing a row does.
- [[Span]] — A stretch of text within a line.
- `pub type Line = Vec<Span>` — A line of text made of spans.
- `pub fn text(text: impl Into<String>) -> Span` — public function, returns `Span`; called by `Cell::text`, `Doc::text`, `Doc::words`, `agents_card` and 25 more.
- `pub fn strong(text: impl Into<String>) -> Span` — public function, returns `Span`; called by `message_page`, `workspace_page`.
- `pub fn code(text: impl Into<String>) -> Span` — public function, returns `Span`; called by `agents_card`, `appearance`, `backend`, `contracts_page` and 14 more.
- `pub fn rich(source: &str) -> Line` — text from the backend, in which backticks mark code: `like this`.
- `pub fn file(path: &str) -> Span` — A link to a source file, labelled with its path.
- `pub fn symbol(qualified: &str) -> Span` — A link to a symbol, labelled with the last part of its name.
- `pub fn note(label: impl Into<String>, target: impl Into<String>) -> Span` — A link to a note, labelled `label`.
- `pub fn plain(line: &[Span]) -> String` — the text of a line with its styling dropped, for tests.
- `pub struct Stat` — one headline number.
- `pub struct Column` — A column of a table.
- `pub enum Cell` — what a table cell holds.
- `pub struct Table` — A table: columns, and rows of one cell for each.
- `pub struct Row` — A row that can be pressed.
- `pub struct Choice` — one choice among a few, shown as a group of toggle buttons.
- `pub struct Press` — one thing to press in a row of buttons.
- [[Block]] — A part of a page.
- [[Doc]] — A generated page.

And 2 more items.

## Depends on

- [[Settings Module]] — uses `Note`
- [[Action]] — uses `Action`
- [[Span]] — uses `Span`
- [[Glyph]] — uses `Glyph`
- [[Tone]] — uses `Tone`

## Used by

- [[Live Module]] — from `file_page`, `file_row`, `hit_target`, `home_page`
- [[Settings Pages Module]] — from `agents_card`, `appearance`, `backend`, `diagnostics`
- [[Pages Module]] — from `contracts_page`, `coupling_page`, `coverage_page`, `dead_code_page`
- [[Doc View Module]] — from `SelectRequest`, `choice_tint`, `choices_row`, `draw_cell`
- [[Doc]] — from `tables`, `text`, `words`
- [[Settings View Module]] — from `OpenMenu`, `lists_themes`
- [[Block]] — from `Block`

## See also

- [[_index]]
- [[codewiki-app]]
