---
tags: [core, openui-lang]
---

# Node

> `Node` enum defined in `openui_lang::eval` with 4 methods.

## Source

- `crates/openui-lang/src/eval.rs` — definition of `Node`

## Items

Declared as `pub enum Node`.

Members:

- `pub fn as_text(&self) -> Option<&str>` — public method, returns `Option<&str>`; called by `text`.
- `pub fn as_list(&self) -> &[Node]` — public method, returns `&[Node]`; called by `list_of`, `review`.
- `fn display(&self) -> String` — text form used by string concatenation.

And 1 more private item with no comment.

## Depends on

- [[Parser Module]] — uses `format_number`

## Used by

- [[Eval Module]] — from `Document`, `binary`, `component`, `eval`
- [[cw-illustrate]] — from `Props`, `element`, `review`, `text`
- [[codewiki-app]] — from `AppView`, `Page`, `open_note_as`
- [[Illustration Module]] — from `Props`, `list_of`, `render`

## See also

- [[_index]]
- [[Eval Module]]
