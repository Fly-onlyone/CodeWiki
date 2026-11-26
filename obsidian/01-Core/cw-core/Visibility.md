---
tags: [core, cw-core]
---

# Visibility

> `Visibility` enum defined in `cw_core::model` with 2 methods.

## What it does

`Visibility` says how far a symbol can be seen: `Public`, `Crate` or `Private`. Each language parser maps its own rules (Rust `pub`, Python underscore names, Java modifiers, TypeScript members) onto these three values, so later code can ask one question about all languages. The dead-code and risk reports use it to tell exported items from internal ones.

## Source

- `crates/cw-core/src/model.rs` — definition of `Visibility`

## How it works

The parsers for Rust, Python, Java and TypeScript each have a `visibility` function that returns a `Visibility` for an item. The value is stored on the [[Symbol]] and written to the index as text with `as_str`. When the index is read back, `symbol_from_row` in [[Store Module]] calls `parse` to restore it.

## Items

Declared as `pub enum Visibility`.

Members:

- `pub fn as_str(self) -> &'static str` — public method, returns `&'static str`.
- `pub fn parse(s: &str) -> Self` — public method, returns `Self`; called by `symbol_from_row`.

## Used by

- [[Typescript Module]] — from `member_visibility`, `visibility`
- [[Store Module]] — from `symbol_from_row`
- [[Java Module]] — from `visibility`
- [[Python Module]] — from `visibility`
- [[Rust Module]] — from `visibility`
- [[Symbol]] — from `Symbol`

## Gotchas

- `parse` never fails. Any text other than `public` or `crate` becomes `Private`, so a misspelled or new value in the index reads as private.
- Only three levels exist. Finer Rust forms such as `pub(super)` or `pub(in path)` must be folded into one of them by the parser.

## See also

- [[_index]]
- [[Model Module]]
