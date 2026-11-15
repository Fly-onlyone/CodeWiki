---
tags: [core, codewiki-app]
---

# Doc

> A generated page.

## Source

- `crates/codewiki-app/src/doc.rs` — definition of `Doc`

## How it works

Declared as `pub struct Doc`.

Public API:

- `pub fn tables(&self) -> Vec<&Table>` — the tables of the page, in order.
- `pub fn headings(&self) -> Vec<&str>` — section headings and card titles, in order.
- `pub fn words(&self) -> String` — all the words of the page, one block to a line.
- `pub fn actions(&self) -> Vec<Action>` — every action a link, row or button of the page carries.
- `pub fn new(title: impl Into<String>) -> Self`
- `pub fn push(&mut self, block: Block)`

## Depends on

- [[Doc Module]] — uses `Table`, `plain`, `text`
- [[Action]] — uses `Action`
- [[Block]] — uses `Block`
- [[Span]] — uses `Span`

## Used by

- [[Pages Module]] — from `contracts_page`, `coupling_page`, `coverage_page`, `dead_code_page`
- [[Live Module]] — from `file_page`, `home_page`, `hotspots_page`, `message_page`
- [[codewiki-app]] — from `Page`, `doc_element`, `home_doc`, `open_global`
- [[Doc View Module]] — from `DocPage`, `draw_strip`, `show`, `strips`

## See also

- [[_index]]
- [[Doc Module]]
