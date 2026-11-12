---
tags: [core, cw-core]
---

# SymbolKind

> `SymbolKind` enum defined in `cw_core::model` with 3 methods.

## Source

- `crates/cw-core/src/model.rs` — definition of `SymbolKind`

## How it works

Declared as `pub enum SymbolKind`.

Public API:

- `pub fn is_type(self) -> bool` — types are the symbols that can earn their own concept note.
- `pub fn parse(s: &str) -> Option<Self>`
- `pub fn as_str(self) -> &'static str`

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
