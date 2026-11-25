---
tags: [core, codewiki-app]
---

# Action

> What following a link or pressing a row does.

## Source

- `crates/codewiki-app/src/doc.rs` — definition of `Action`

## Items

Declared as `pub enum Action`.

## Depends on

- [[MotionPref]] — uses `MotionPref`
- [[Category]] — uses `Category`
- [[FieldId]] — uses `FieldId`

## Used by

- [[Doc View Module]] — from `LinkedLine`, `OnAction`, `choices_row`, `line`
- [[Doc Module]] — from `Choice`, `Press`, `Row`, `aside`
- [[codewiki-app]] — from `run_action`, `run_settings_action`
- [[Doc]] — from `actions`, `of_line`
- [[Live Module]] — from `HitTarget`
- [[Toasts Module]] — from `Notice`
- [[Block]] — from `Block`
- [[Span]] — from `Span`

## See also

- [[_index]]
- [[Doc Module]]
