---
tags: [core, cw-parse]
---

# ParsedFile

> `ParsedFile` struct defined in `cw_parse::rust`.

## What it does

`ParsedFile` is what a language extractor hands back for one source file. It is a bundle of lists: the symbols declared, the `use` imports, the calls, the impl and inheritance facts, the module declarations, the type mentions and the value mentions. It is plain data, with no behavior. The extractors for Rust, Python, TypeScript and Java all return this one shape, so everything after them works the same for every language.

## Source

- `crates/cw-parse/src/rust.rs` — definition of `ParsedFile`

## How it works

An extractor fills the lists while it walks one syntax tree. [[Scan Module]] collects one `ParsedFile` per file and keeps the symbols. [[Lang Module]] then rewrites the imports of Python and Java files. After that the [[Resolver]] reads all the files together. It matches each call, type mention and import to a known symbol and produces the edges of the graph.

The lists hold raw, unresolved paths as they were written in the source. For example a call stores the path segments of the callee and the module it was made in. Name lookup is the Resolver's job, not the extractor's.

## Items

Declared as `pub struct ParsedFile`.

## Depends on

- [[Rust Module]] — uses `CallSite`, `ImplFact`, `ModDecl`, `TypeRef`
- [[Symbol]] — uses `Symbol`
- [[UseImport]] — uses `UseImport`

## Used by

- [[Resolver]] — from `new`, `resolve`, `see_c_modules`
- [[Cfamily Module]] — from `includes_of`, `merge`
- [[Lang Module]] — from `link_imports`, `modules_of`
- [[Rust Module]] — from `Extractor`, `parse_rust`
- [[Typescript Module]] — from `parse_typescript`, `parse_typescript_with`
- [[Sink]] — from `Sink`, `new`
- [[C Module]] — from `parse_c`
- [[Csharp Module]] — from `parse_csharp`

## Gotchas

- `has_errors` is true when the parser met syntax errors. The facts are then probably incomplete, and the scan lists such files in `files_with_errors`.
- The struct derives `Default`, and extractors start from the default and set `has_errors` only. Every list starts empty.
- Nothing here is checked against the code that was parsed. A name in a list can point to something that does not exist, and the Resolver simply finds no match.

## See also

- [[_index]]
- [[Rust Module]]
