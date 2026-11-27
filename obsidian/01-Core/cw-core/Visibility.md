---
tags: [core, cw-core]
---

# Visibility

> `Visibility` enum defined in `cw_core::model` with 2 methods.

## Source

- `crates/cw-core/src/model.rs` — definition of `Visibility`

## How it works

1. `symbol_from_row` in [[Store Module]] calls `parse`.

## Items

Declared as `pub enum Visibility`.

Members:

- `pub fn as_str(self) -> &'static str` — public method, returns `&'static str`.
- `pub fn parse(s: &str) -> Self` — public method, returns `Self`; called by `symbol_from_row`.

## Used by

- [[Csharp Module]] — from `Ctx`, `visibility`
- [[Typescript Module]] — from `member_visibility`, `visibility`
- [[Store Module]] — from `symbol_from_row`
- [[C Module]] — from `visibility`
- [[Go Module]] — from `visibility`
- [[Java Module]] — from `visibility`
- [[Kotlin Module]] — from `visibility`
- [[Python Module]] — from `visibility`

## See also

- [[_index]]
- [[Model Module]]
