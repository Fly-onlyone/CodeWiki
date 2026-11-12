---
tags: [core, cw-parse]
---

# Sink

> Collects the facts of one file.

## Source

- `crates/cw-parse/src/lang.rs` — definition of `Sink`

## How it works

Declared as `pub(crate) struct Sink<'a>`.

Public API:

- `pub fn value(&mut self, owner: &str, path: Vec<String>)` — A name used as a value inside `owner`.
- `pub fn symbol(&mut self, scope: &str, name: &str, kind: SymbolKind, span: Node) -> &mut Symbol` — add a public symbol named `name` under `scope` covering `span`, and return it so the caller can fill in the remaining fields.
- `pub fn type_ref(&mut self, owner: &str, path: Vec<String>)` — A type mentioned inside `owner`.
- `pub fn inherits(&mut self, qualified: &str, parent: Vec<String>)` — `qualified` extends or implements the type written as `parent`.
- `pub fn module_symbol( &mut self, signature: String, doc: String, start_line: u32, end_line: u32, )` — record the module the file defines, spanning `start_line..=end_line`.
- `pub fn call(&mut self, owner: &str, path: Vec<String>, method: bool, self_type: Option<&str>)` — A call made by `owner`.

## Depends on

- [[Rust Module]] — uses `CallSite`, `ImplFact`, `TypeRef`, `ValueRef`
- [[Lang Module]] — uses `is_generic_param`, `split_qualified`
- [[Symbol]] — uses `Symbol`
- [[SymbolKind]] — uses `SymbolKind`
- [[ParsedFile]] — uses `ParsedFile`
- [[UseImport]] — uses `UseImport`

## Used by

- [[Typescript Module]] — from `Extractor`, `class`, `enumeration`, `export`
- [[Java Module]] — from `Extractor`, `enum_constant`, `field`, `method`
- [[Python Module]] — from `Extractor`, `attribute`, `class`, `function`
- [[codewiki-app]] — from `edit_view`, `new`, `palette_matches`, `save_edit`

## See also

- [[_index]]
- [[Lang Module]]
