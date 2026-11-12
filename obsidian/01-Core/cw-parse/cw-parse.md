---
tags: [core, cw-parse]
---

# cw-parse

> Source parsing and symbol extraction for CodeWiki.

## Source

- `crates/cw-parse/Cargo.toml` — package manifest
- `crates/cw-parse/src/lib.rs` — crate root

## How it works

Builds a library.

Modules:

- [[Java Module]] — Java symbol and fact extraction.
- [[Lang Module]] — plumbing shared by the Python, TypeScript and Java extractors: the fact sink, text helpers, the file-to-module mapping and import classification.
- [[Python Module]] — Python symbol and fact extraction.
- [[Resolve Module]] — name resolution without a compiler.
- [[Rust Module]] — Rust symbol and fact extraction.
- [[Scan Module]] — walk a repository and produce every fact the index needs.
- [[Typescript Module]] — TypeScript and TSX symbol and fact extraction.
- [[Workspace Module]] — Cargo package discovery and the file-to-module mapping.

## See also

- [[_index]]
- [[Crate Dependency Flow]]
