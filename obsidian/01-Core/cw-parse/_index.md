---
tags: [moc, core, cw-parse]
---

# Core / cw-parse — Map of Content

> Notes for `cw-parse`: 13 concepts.

## Notes

- [[cw-parse]] — source parsing and symbol extraction for CodeWiki.
- [[Java Module]] — Java symbol and fact extraction.
- [[Lang Module]] — plumbing shared by the Python, TypeScript and Java extractors: the fact sink, text helpers, the file-to-module mapping and import classification.
- [[ParsedFile]] — `ParsedFile` struct defined in `cw_parse::rust`.
- [[Python Module]] — Python symbol and fact extraction.
- [[Resolve Module]] — name resolution without a compiler.
- [[Resolver]] — `Resolver` struct defined in `cw_parse::resolve` with 7 methods.
- [[Rust Module]] — Rust symbol and fact extraction.
- [[Scan Module]] — walk a repository and produce every fact the index needs.
- [[Sink]] — collects the facts of one file.
- [[Typescript Module]] — TypeScript and TSX symbol and fact extraction.
- [[UseImport]] — one name brought into scope by a `use` declaration.
- [[Workspace Module]] — Cargo package discovery and the file-to-module mapping.

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]
