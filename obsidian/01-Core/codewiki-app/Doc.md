---
tags: [core, codewiki-app]
---

# Doc

> A generated page.

## Source

- `crates/codewiki-app/src/doc.rs` — definition of `Doc`

## How it works

1. `file_page` and `home_page` in [[Live Module]], and 19 more, call `new`.

## Items

Declared as `pub struct Doc`.

Members:

- `pub fn new(title: impl Into<String>) -> Self` — public method, returns `Self`; called by `contracts_page`, `coupling_page`, `coverage_page`, `dead_code_page` and 17 more.
- `pub fn lead(mut self, lead: impl Into<String>) -> Self` — public method, returns `Self`.
- `pub fn push(&mut self, block: Block)` — public method; called by `Doc::actions`, `Doc::heading`, `Doc::text`, `Doc::words` and 2 more.
- `pub fn heading(&mut self, title: impl Into<String>)` — public method.
- `pub fn text(&mut self, line: impl Into<String>)` — public method; called by `Doc::words`.
- `pub fn flat(&self) -> Vec<&Block>` — every block, with the bodies of cards opened out.
- `pub fn tables(&self) -> Vec<&Table>` — the tables of the page, in order.
- `pub fn headings(&self) -> Vec<&str>` — section headings and card titles, in order.
- `pub fn words(&self) -> String` — all the words of the page, one block to a line.
- `pub fn actions(&self) -> Vec<Action>` — every action a link, row or button of the page carries.
- `pub fn fields(&self) -> Vec<FieldId>` — the fields the page asks the user to type into, in order. A locked one is listed too: its input shows the value, and takes none.

## Depends on

- [[Doc Module]] — uses `Table`, `note`, `plain`, `text`
- [[Action]] — uses `Action`
- [[Block]] — uses `Block`
- [[Span]] — uses `Span`
- [[FieldId]] — uses `FieldId`

## Used by

- [[Pages Module]] — from `contracts_page`, `coupling_page`, `coverage_page`, `dead_code_page`
- [[Live Module]] — from `file_page`, `home_page`, `hotspots_page`, `message_page`
- [[codewiki-app]] — from `Page`, `doc_element`, `home_doc`, `open_global`
- [[Doc View Module]] — from `DocPage`, `column`, `draw_strip`, `show`
- [[Settings Pages Module]] — from `page`

## See also

- [[_index]]
- [[Doc Module]]
