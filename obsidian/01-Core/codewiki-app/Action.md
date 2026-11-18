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
- [[Category]] — uses `Category`
- [[FieldId]] — uses `FieldId`

## Used by

- [[Doc Module]] — from `Choice`, `Press`, `Row`, `new`
- [[Doc View Module]] — from `OnAction`, `choices_row`, `line`, `secret_control`
- [[codewiki-app]] — from `run_action`, `run_settings_action`
- [[Doc]] — from `actions`, `of_line`
- [[Block]] — from `Block`
- [[Span]] — from `Span`

## See also

- [[_index]]
- [[Doc Module]]
