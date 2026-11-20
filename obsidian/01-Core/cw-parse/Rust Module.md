---
tags: [core, cw-parse, module]
---

# Rust Module

> Rust symbol and fact extraction.

## What it does

Written against the node kinds and field names published in the grammar's own `node-types.json` (tree-sitter-rust 0.24). No query files are used.

## Source

- `crates/cw-parse/src/rust.rs` — module `cw_parse::rust`

## How it works

1. `scan_repo` in [[Scan Module]] calls `parse_rust`. `parse_rust` calls `Error::external`, then `Error::invalid` (both in [[Error]]).

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

## See also

- [[_index]]
- [[cw-parse]]
