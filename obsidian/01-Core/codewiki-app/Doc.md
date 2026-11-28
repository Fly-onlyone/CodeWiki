---
tags: [core, codewiki-app]
---

# Doc

> A generated page.

## What it does

`Doc` is one generated page of the app: a title, an optional lead line under it, and a list of [[Block]]s. A page function in the live or settings code turns backend data into a `Doc`. The doc view then draws it. `Doc` holds no drawing code, so a test can build a page and read it without a window.

## Source

- `crates/codewiki-app/src/doc.rs` — definition of `Doc`

## How it works

A page starts with `Doc::new(title)`, optionally `lead`, and then fills itself with `push`, or with the shortcuts `heading` and `text`. The blocks stay in order. A [[Block]] such as a card holds its own list of blocks, so a page is a tree.

Six methods exist only in tests and read a page back:

- `flat` lists every block, with card bodies opened out.
- `tables`, `headings` and `words` pick out tables, headings and all visible text, one block to a line.
- `actions` lists every [[Action]] a link, row (and its aside buttons), button, choice, unlocked switch or select, or secret control carries. Disabled buttons are left out.
- `fields` lists the inputs the page asks the user to fill in.

Page tests use these to check what a page says and offers. The [[Doc View Module]] draws the same tree in the app.

## Items

Declared as `pub struct Doc`.

Members:

- `pub fn new(title: impl Into<String>) -> Self` — public method, returns `Self`; called by `contracts_page`, `coupling_page`, `coverage_page`, `dead_code_page` and 20 more.
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

- [[Live Module]] — from `file_page`, `home_page`, `hotspots_page`, `message_page`
- [[Pages Module]] — from `contracts_page`, `coupling_page`, `coverage_page`, `dead_code_page`
- [[codewiki-app]] — from `Page`, `doc_element`, `home_doc`, `open_global`
- [[Doc View Module]] — from `DocPage`, `column`, `draw_strip`, `show`
- [[Settings Pages Module]] — from `page`

## Gotchas

- `flat`, `tables`, `headings`, `words`, `actions` and `fields` exist only under test. They are not in the shipped app.
- A locked switch, select or secret offers no action in `actions`, because nothing can be done there. A disabled button is skipped too. A locked field is still listed in `fields`, since its input shows the value.
- A secret lists an input in `fields` only while empty or being replaced. A saved one lists replace and remove actions instead.
- `Doc` is `Default`, with an empty title, so a blank page is easy to create by accident.

## See also

- [[_index]]
- [[Doc Module]]
