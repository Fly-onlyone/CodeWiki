---
tags: [core, codewiki-app]
---

# Span

> A stretch of text within a line.

## What it does

`Span` is a stretch of text inside one line of a generated page. A line is a `Vec<Span>`. A span is plain text, strong text, code, or a link. A link carries an [[Action]] that says what a click does, such as opening a note, a symbol or a file. It is a plain data type with no drawing code; the [[Doc View Module]] draws it.

## Source

- `crates/codewiki-app/src/doc.rs` — definition of `Span`

## How it works

Small helpers in the [[Doc Module]] build spans: `text`, `strong`, `code`, and the link makers `file`, `symbol` and `note`. `file` labels a link with its path. `symbol` labels it with the last part of the qualified name. Both set `code: true`, so the label is set in the code font. `note` takes any label and sets `code: false`.

`rich` turns backend text into a line. It splits at backticks, so every second part, between two backticks, becomes a code span and the rest plain text. Empty parts are dropped.

A table cell can wrap a span or a whole line through `From<Span>` and `From<Line>`.

## Items

Declared as `pub enum Span`.

## Depends on

- [[Action]] — uses `Action`

## Used by

- [[Doc Module]] — from `Line`, `code`, `file`, `from`
- [[Doc View Module]] — from `line`
- [[Settings Pages Module]] — from `serve_help`
- [[Doc]] — from `of_line`

## Gotchas

- `rich` has no escape for a backtick. An odd number of backticks leaves the last part as code.
- A link's `code` flag only changes the font. What a click does comes from its `Action`.
- Code that scans a page for actions must look into link spans too. The test helper `actions` does this with `of_line`.

## See also

- [[_index]]
- [[Doc Module]]
