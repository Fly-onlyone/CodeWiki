---
tags: [core, cw-parse, module]
---

# Rust Module

> Rust symbol and fact extraction.

## What it does

This module reads one Rust source file and returns a [[ParsedFile]]. It also defines the fact types that every language shares: [[UseImport]], `CallSite`, `ImplFact`, `ModDecl`, `TypeRef` and `ValueRef`. It runs on the tree-sitter syntax tree and is written against the node names in the grammar's `node-types.json`. No query files are used.

## Source

- `crates/cw-parse/src/rust.rs` — module `cw_parse::rust`

## How it works

`parse_rust` takes the file path, the qualified module path the file defines (such as `cw_core::hash`) and the text. It parses the file and walks the top-level items with `visit_item`.

For each function, struct, enum, union, trait, impl, `mod`, `use`, const, static, type alias and macro definition, `push_symbol` records a symbol. The symbol has a qualified name, visibility, line span, a one-line signature, `///` doc lines and attribute names read from the nodes just above it.

Inside a function body `walk_body` records calls, type mentions and names used as values. Items declared inside a body are extracted too, under the function's scope. `scan_tokens` handles macro arguments, which tree-sitter leaves as raw tokens: it rebuilds paths such as `a::b(...)` and `.name(...)`. `format_captures` adds names captured by format strings like `{name}`. `flatten_use` turns a `use` tree with braces into one `UseImport` for each name.

## Items

- [[UseImport]] — one name brought into scope by a `use` declaration.
- `pub struct CallSite` — A call found in a function or method body.
- `pub struct ImplFact` — `impl Trait for Type` or an inherent `impl Type`.
- `pub struct ModDecl` — `mod name;` (file module) or `mod name { .. }` (inline).
- `pub struct TypeRef` — A type mentioned inside a symbol's declaration or body.
- `pub struct ValueRef` — A name used as a value, or called inside a macro's arguments.
- [[ParsedFile]] — struct
- `pub fn parse_rust(file: &str, module_path: &str, source: &str) -> Result<ParsedFile>` — parse one Rust source file. `module_path` is the qualified module the file defines, for example `cw_core::hash`.
- `fn format_captures(literal: &str) -> Vec<String>` — identifiers captured inline by a format string: `{name}`, `{name:>5}`.
- `fn quoted_paths(attribute: &str) -> Vec<Vec<String>>` — quoted strings in an attribute that look like paths: `"a::b"`.
- `fn split_path(text: &str) -> Vec<String>` — split `a::b::<T>::c` style text into plain segments.

And 2 more private items with no comment.

## Depends on

- [[Error]] — uses `external`, `invalid`
- [[Error Module]] — uses `Result`
- [[Model Module]] — uses `Visibility`
- [[Symbol]] — uses `Symbol`
- [[SymbolKind]] — uses `SymbolKind`
- [[ParsedFile]] — uses `ParsedFile`
- [[UseImport]] — uses `UseImport`

## Used by

- [[Sink]] — from `call`, `inherits`, `type_ref`, `value`
- [[Lang Module]] — from `link_imports`
- [[Scan Module]] — from `scan_repo`
- [[ParsedFile]] — from `ParsedFile`

## Gotchas

- A value reference is recorded once for each owner and path. Locals are recorded too and just fail to resolve later.
- Calls are pushed in tree order, so `x.first().second()` is recorded as `second`, then `first`. Use the `at` byte offset to get source order.
- Items in a trait take the trait's visibility, since they carry no modifier. `pub(crate)` and any other restricted form become `Crate`.
- A signature is cut at 300 characters.
- Calls inside macro arguments are found by token pattern, not by parsing. Unusual macro syntax can be missed.

## See also

- [[_index]]
- [[cw-parse]]
