---
tags: [core, codewiki-app]
---

# Action

> What following a link or pressing a row does.

## Source

- `crates/codewiki-app/src/doc.rs` — definition of `Action`

## How it works

Declared as `pub enum Action`.

## Depends on

- [[MotionPref]] — uses `MotionPref`

## Used by

- [[Doc Module]] — from `Choice`, `Row`, `new`
- [[Doc View Module]] — from `OnAction`, `choices_row`, `line`
- [[Doc]] — from `actions`, `of_line`
- [[codewiki-app]] — from `run_action`
- [[Block]] — from `Block`
- [[Span]] — from `Span`

## See also

- [[_index]]
- [[Doc Module]]
