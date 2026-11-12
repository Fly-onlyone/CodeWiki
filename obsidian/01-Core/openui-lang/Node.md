---
tags: [core, openui-lang]
---

# Node

> `Node` enum defined in `openui_lang::eval` with 4 methods.

## Source

- `crates/openui-lang/src/eval.rs` — definition of `Node`

## How it works

Declared as `pub enum Node`.

Public API:

- `pub fn as_list(&self) -> &[Node]`
- `pub fn as_text(&self) -> Option<&str>`

## Depends on

- [[Parser Module]] — uses `format_number`

## Used by

- [[Eval Module]] — from `Document`, `binary`, `component`, `eval`
- [[Illustration Module]] — from `Props`, `children`, `label`, `list_of`
- [[codewiki-app]] — from `AppView`, `Page`
- [[codewiki]] — from `run`

## See also

- [[_index]]
- [[Eval Module]]
