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

- `pub fn as_text(&self) -> Option<&str>` — public method, returns `Option<&str>`.
- `pub fn as_list(&self) -> &[Node]` — public method, returns `&[Node]`; called by `list_of`.
- `fn display(&self) -> String` — text form used by string concatenation.

And 1 more private item with no comment.

## Depends on

- [[Parser Module]] — uses `format_number`

## Used by

- [[Eval Module]] — from `Document`, `binary`, `component`, `eval`
- [[Illustration Module]] — from `Props`, `children`, `label`, `list_of`
- [[codewiki-app]] — from `AppView`, `Page`

## See also

- [[_index]]
- [[Eval Module]]
