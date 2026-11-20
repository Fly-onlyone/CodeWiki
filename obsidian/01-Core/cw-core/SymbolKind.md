---
tags: [core, cw-core]
---

# SymbolKind

> `SymbolKind` enum defined in `cw_core::model` with 3 methods.

## Source

- `crates/cw-core/src/model.rs` — definition of `SymbolKind`

## How it works

1. `symbol_from_row` in [[Store Module]] calls `parse`.

## Items

Declared as `pub enum SymbolKind`.

Members:

- `pub fn as_str(self) -> &'static str` — public method, returns `&'static str`.
- `pub fn parse(s: &str) -> Option<Self>` — public method, returns `Option<Self>`; called by `symbol_from_row`.
- `pub fn is_type(self) -> bool` — types are the symbols that can earn their own concept note.

## Used by

- [[Rust Module]] — from `push_symbol`, `simple_item`, `type_item`
- [[Refactor Module]] — from `extract_types`, `split_files`
- [[Resolver]] — from `by_bare_name`, `resolve`
- [[Store Module]] — from `symbol_from_row`
- [[Health Module]] — from `file_health`
- [[Plan Module]] — from `plan`
- [[Symbol]] — from `Symbol`
- [[Sink]] — from `symbol`

## See also

- [[_index]]
- [[Model Module]]
