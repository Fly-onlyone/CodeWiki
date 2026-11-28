---
tags: [core, cw-core]
---

# Visibility

> `Visibility` enum defined in `cw_core::model` with 2 methods.

## What it does

`Visibility` says who can see a symbol: `Public`, `Crate` or `Private`. Every `Symbol` carries one. The language parsers set it from each language's own rules (for example `pub`, `internal`, or a leading underscore). The store keeps it as text, and tools such as dead-code search use it to judge how widely a symbol can be reached.

## Source

- `crates/cw-core/src/model.rs` — definition of `Visibility`

## How it works

`as_str` turns the value into the text `public`, `crate` or `private`. The store writes that text into its table. `parse` does the reverse when `symbol_from_row` reads a row back. The language modules in `cw-parse` build the enum directly in their own `visibility` helpers.

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

## Gotchas

- `parse` never fails. Any text other than `public` or `crate` becomes `Private`, so a misspelled or new value is silently read as private.
- `Crate` is a middle level. Languages with no such idea (Go, Python) map onto `Public` and `Private` only.

## See also

- [[_index]]
- [[Model Module]]
