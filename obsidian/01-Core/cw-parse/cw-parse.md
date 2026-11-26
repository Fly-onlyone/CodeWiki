---
tags: [core, cw-parse]
---

# cw-parse

> Source parsing and symbol extraction for CodeWiki.

## What it does

cw-parse turns a repository's source files into facts: symbols, imports, calls, type references and Cargo package data. It uses tree-sitter grammars for Rust, Python, TypeScript and Java, and needs no compiler. The index crate stores what it returns. The wiki, the dependency graph and the risk tools all rest on these facts.

## Source

- `crates/cw-parse/Cargo.toml` — package manifest
- `crates/cw-parse/src/lib.rs` — crate root

## How it works

The crate is a library with one module per language. `lib.rs` only declares them and re-exports the main entry points. `scan_repo` in [[Scan Module]] is the front door. It lists files with `list_files`, reads each `Cargo.toml` through [[Workspace Module]], and sends each source file to its parser: `parse_rust`, `parse_python`, `parse_typescript` or `parse_java`. It returns a `RepoFacts` value.

The parsers only see one file at a time, so names stay unresolved. [[Resolve Module]] then links a call or type name to the symbol it means, using imports and the crate layout. [[Lang Module]] holds helpers shared by the Python, TypeScript and Java extractors.

## Items

Modules:

- [[Java Module]] — Java symbol and fact extraction.
- [[Lang Module]] — plumbing shared by the Python, TypeScript and Java extractors: the fact sink, text helpers, the file-to-module mapping and import classification.
- [[Python Module]] — Python symbol and fact extraction.
- [[Resolve Module]] — name resolution without a compiler.
- [[Rust Module]] — Rust symbol and fact extraction.
- [[Scan Module]] — walk a repository and produce every fact the index needs.
- [[Typescript Module]] — TypeScript and TSX symbol and fact extraction.
- [[Workspace Module]] — Cargo package discovery and the file-to-module mapping.

## Gotchas

- Resolution works without a compiler. It is a best guess from imports and names, so macros, trait dispatch and generated code can stay unresolved.
- Each language module extracts what it can from the syntax tree. Code that fails to parse well gives fewer facts, not an error.

## See also

- [[_index]]
- [[Crate Dependency Flow]]
