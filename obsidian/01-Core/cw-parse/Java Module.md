---
tags: [core, cw-parse, module]
---

# Java Module

> Java symbol and fact extraction.

## What it does

This module reads one Java file and turns it into facts: the package, classes, interfaces, enums, records, methods and constants, plus which names each one calls, uses or extends. It is the Java front end of the scanner. It works on the syntax tree from tree-sitter, using the node kinds in the grammar's published `node-types.json`. No query files are used.

## Source

- `crates/cw-parse/src/java.rs` — module `cw_parse::java`

## How it works

`parse_java` builds the tree and reads the `package` line. The package becomes the module name, with `::` between parts. A file with no package is named by its directory (`java_default_package`). It records the module as a symbol, then walks the top-level nodes. An import goes to `Extractor::import`; anything else goes to `type_decl`.

`type_decl` makes a symbol for the type, with its signature, Javadoc and annotations. It collects the member names of the body first. Then it records `extends` and `implements` types as inheritance, and walks the body through `members`. Methods, constructors, constants and enum constants each get a symbol. Plain fields give no symbol and only add the types and values they mention.

`walk` records what code mentions. `invocation` decides what a call looks like: a bare call is a member of an enclosing type or a static import; `this.x()` carries the class as `self_type`; `Type.x()` is a path; any other receiver is a method call of unknown type. Every fact goes into a [[Sink]]. Imports are stored as written. [[Scan Module]] decides later which ones are repository code.

## Items

- `pub fn parse_java(file: &str, source: &str) -> Result<ParsedFile>` — parse one Java source file. The module is the file's `package`, for example `com::acme::text`; a file without one is named by its directory. Imports are recorded as written; the scanner decides afterwards which of them name repository packages.
- `struct Ctx` — private struct; used by `Extractor::children`, `Extractor::enum_constant`, `Extractor::field`, `Extractor::invocation` and 6 more.
- `struct Modifiers` — private struct; used by `Extractor::modifiers`, `Extractor::signature`.
- `struct Extractor<'a>` — private struct; used by `parse_java`.
- `fn is_type_decl(kind: &str) -> bool` — private function, returns `bool`; called by `Extractor::members`.

## Depends on

- [[Lang Module]] — uses `block_doc`, `collapse`, `dotted`, `field_all`
- [[Sink]] — uses `Sink`, `between`, `inherits`, `module_symbol`
- [[Error]] — uses `external`, `invalid`
- [[Error Module]] — uses `Result`
- [[Model Module]] — uses `Visibility`
- [[ParsedFile]] — uses `ParsedFile`

## Used by

- [[Scan Module]] — from `scan_repo`

## Gotchas

- Only capitalized names count as value references. Java keeps methods and variables apart, so a lowercase name in an expression is a variable, and recording it would invent links.
- Visibility is mapped to the Rust-like scale. `protected` and package-private both become `Crate`. Members of an interface are `Public` unless marked.
- Plain instance fields make no symbol. Only `static final` fields and interface constants do.
- Inherited methods are not followed. A bare call finds only members of an enclosing type or a static import.
- A call on an unknown receiver, `super` included, is recorded by method name only, so the resolver may guess.

## See also

- [[_index]]
- [[cw-parse]]
