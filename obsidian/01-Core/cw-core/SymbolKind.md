---
tags: [core, cw-core]
---

# SymbolKind

> `SymbolKind` enum defined in `cw_core::model` with 3 methods.

## What it does

`SymbolKind` names what sort of code item a symbol is: module, function, method, struct, enum, union, trait, impl, const, static, type alias, macro, class or interface. The parser sets it for every item it finds, and the index, the graph and the wiki read it. `Class` and `Interface` cover Python, TypeScript and Java, so one enum serves all indexed languages.

## Source

- `crates/cw-core/src/model.rs` — definition of `SymbolKind`

## How it works

The kind is stored in the index as text. `as_str` gives the stored name, such as `type_alias`. `parse` turns stored text back into a kind; `symbol_from_row` in [[Store Module]] calls it when it rebuilds a [[Symbol]] from a database row. `is_type` is true for struct, enum, union, trait, class and interface. Code that decides which items earn their own concept note, such as the planner and the health check, uses it as the filter.

The serde form is snake_case, so JSON output and the stored text use the same names.

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

## Gotchas

- `parse` returns `Option` and gives `None` for an unknown name. [[Visibility]] and the other enums in the model fall back to a default instead. A caller must handle the `None`.
- Adding a variant needs three edits: the enum, `as_str` and `parse`. The round-trip test lists only some variants, so a missed arm may not fail a test.
- `Impl`, `Const`, `Static`, `Macro` and `Module` are not types by `is_type`, so they get no concept note of their own.

## See also

- [[_index]]
- [[Model Module]]
