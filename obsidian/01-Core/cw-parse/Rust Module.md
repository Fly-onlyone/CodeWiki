---
tags: [core, cw-parse, module]
---

# Rust Module

> Rust symbol and fact extraction.

## Source

- `crates/cw-parse/src/rust.rs` — module `cw_parse::rust`

## How it works

Main items:

- [[ParsedFile]] — struct
- [[UseImport]] — one name brought into scope by a `use` declaration.
- `ValueRef` — A name used as a value, or called inside a macro's arguments.
- `CallSite` — A call found in a function or method body.
- `TypeRef` — A type mentioned inside a symbol's declaration or body.
- `ImplFact` — `impl Trait for Type` or an inherent `impl Type`.
- `ModDecl` — `mod name;` (file module) or `mod name { .. }` (inline).
- `parse_rust` — parse one Rust source file.

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
