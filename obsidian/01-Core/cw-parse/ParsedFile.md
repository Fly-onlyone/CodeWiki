---
tags: [core, cw-parse]
---

# ParsedFile

> `ParsedFile` struct defined in `cw_parse::rust`.

## Source

- `crates/cw-parse/src/rust.rs` — definition of `ParsedFile`

## How it works

Declared as `pub struct ParsedFile`.

## Depends on

- [[Rust Module]] — uses `CallSite`, `ImplFact`, `ModDecl`, `TypeRef`
- [[Symbol]] — uses `Symbol`
- [[UseImport]] — uses `UseImport`

## Used by

- [[Lang Module]] — from `link_imports`, `modules_of`
- [[Rust Module]] — from `Extractor`, `parse_rust`
- [[Sink]] — from `Sink`, `new`
- [[Resolver]] — from `new`, `resolve`
- [[Java Module]] — from `parse_java`
- [[Python Module]] — from `parse_python`
- [[Scan Module]] — from `scan_repo`
- [[Typescript Module]] — from `parse_typescript`

## See also

- [[_index]]
- [[Rust Module]]
