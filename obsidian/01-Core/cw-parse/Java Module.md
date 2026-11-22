---
tags: [core, cw-parse, module]
---

# Java Module

> Java symbol and fact extraction.

## What it does

Written against the node kinds and field names published in the grammar's own `node-types.json` (tree-sitter-java 0.23). No query files are used.

## Source

- `crates/cw-parse/src/java.rs` — module `cw_parse::java`

## How it works

1. `scan_repo` in [[Scan Module]] calls `parse_java`. `parse_java` calls `Error::external`, then `Error::invalid` (both in [[Error]]), then `named`, then `dotted`, then `java_default_package` (all in [[Lang Module]]), then `Sink::new` in [[Sink]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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
- [[Visibility]] — uses `Visibility`
- [[ParsedFile]] — uses `ParsedFile`

## Used by

- [[Scan Module]] — from `scan_repo`

## See also

- [[_index]]
- [[cw-parse]]
