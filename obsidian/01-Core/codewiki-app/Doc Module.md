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

A page function builds a [[Doc]] from backend data with these pieces.

- Text is a [[Span]] (`text`, `strong`, `code`) or a link span (`file`, `symbol`, `note`) that carries an [[Action]]. A `Line` is a list of spans. `rich` turns backend text into a line: every second part between backticks becomes code.
- Tables are built from `Column::new` or `Column::numeric` and `Table::row`, with one `Cell` per column. A cell is a line, a chip with a tone, or a share bar.
- `Stat` is a headline number with an optional hint and a tone. `Row` is a pressable row with an icon, a detail and an optional aside button. `Choice` and `Press` feed choice groups and button rows.
- `SecretState` says whether a secret setting is empty, being replaced, saved (with its last characters) or unreadable. `text` of it gives the line the page shows in place of the input.
- [[Block]] is the part of a page that holds all of these.

The doc view in [[Doc View Module]] draws the result.

## Items

- [[Action]] — what following a link or pressing a row does.
- [[Span]] — A stretch of text within a line.
- `pub type Line = Vec<Span>` — A line of text made of spans.
- `pub fn text(text: impl Into<String>) -> Span` — public function, returns `Span`; called by `Cell::text`, `Doc::text`, `Doc::words`, `agents_card` and 25 more.
- `pub fn strong(text: impl Into<String>) -> Span` — public function, returns `Span`; called by `message_page`, `review_empty_page`, `workspace_page`.
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
- [[Doc View Module]] — from `SelectRequest`, `choice_tint`, `choices_row`, `draw_cell`
- [[Pages Module]] — from `contracts_page`, `coupling_page`, `coverage_page`, `dead_code_page`
- [[Doc]] — from `tables`, `text`, `words`
- [[Settings View Module]] — from `OpenMenu`, `lists_themes`
- [[Block]] — from `Block`

## Gotchas

- `Table::row` checks the cell count against the column count only with a debug assertion. A release build draws a ragged row without a warning.
- `rich` has no escape for a backtick. A text with an odd number of backticks turns its tail into code.
- `symbol` labels a link with the last part of the qualified name only, so two symbols with the same last part look alike. The link target keeps the full name.
- `plain` and `Cell::plain` exist only in tests.

## See also

- [[_index]]
- [[codewiki-app]]
