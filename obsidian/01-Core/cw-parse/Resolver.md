---
tags: [core, cw-parse]
---

# Resolver

> `Resolver` struct defined in `cw_parse::resolve` with 7 methods.

## Source

- `crates/cw-parse/src/resolve.rs` — definition of `Resolver`

## How it works

Declared as `pub struct Resolver<'a>`.

Public API:

- `pub fn lookup( &self, path: &[String], module: &str, self_type: Option<&str>, ) -> Option<&'a Symbol>` — resolve a path as written at a use site inside `module`.
- `pub fn lookup_from( &self, path: &[String], module: &str, self_type: Option<&str>, from: &str, ) -> Option<&'a Symbol>` — `Self::lookup`, then the scopes enclosing `from`: an item declared inside a function is visible to the code around it.
- `pub fn resolve(&self, files: &[(String, ParsedFile)]) -> Resolved` — build every edge for the repository.
- `pub fn new( symbols: &'a [Symbol], files: &[(String, ParsedFile)], crate_names: impl IntoIterator<Item = &'a str>, ) -> Self`

## Depends on

- [[Resolve Module]] — uses `COMMON_METHODS`, `ExternalUse`, `MAX_REEXPORT_DEPTH`, `Resolved`
- [[Model Module]] — uses `Edge`, `EdgeKind`, `file_node`
- [[Language]] — uses `Language`, `from_path`
- [[SymbolKind]] — uses `SymbolKind`, `is_type`
- [[Confidence]] — uses `Confidence`
- [[Symbol]] — uses `Symbol`
- [[ParsedFile]] — uses `ParsedFile`

## Used by

- [[Scan Module]] — from `scan_repo`

## See also

- [[_index]]
- [[Resolve Module]]
