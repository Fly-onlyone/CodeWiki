---
tags: [core, cw-parse]
---

# Sink

> Collects the facts of one file.

## Source

- `crates/cw-parse/src/lang.rs` — definition of `Sink`

## How it works

1. `parse_java` in [[Java Module]] and `parse_python` in [[Python Module]], and 1 more, call `new`.

## Items

Declared as `pub(crate) struct Sink<'a>`.

Members:

- `pub fn new(file: &'a str, source: &'a str, module: &str, has_errors: bool) -> Self` — public method, returns `Self`; called by `parse_java`, `parse_python`, `parse_typescript`.
- `pub fn text(&self, node: Node) -> &'a str` — public method, returns `&'a str`.
- `pub fn between(&self, start: usize, end: usize) -> &'a str` — public method, returns `&'a str`.
- `pub fn module_symbol( &mut self, signature: String, doc: String, start_line: u32, end_line: u32, )` — record the module the file defines, spanning `start_line..=end_line`.
- `pub fn symbol(&mut self, scope: &str, name: &str, kind: SymbolKind, span: Node) -> &mut Symbol` — add a public symbol named `name` under `scope` covering `span`, and return it so the caller can fill in the remaining fields.
- `pub fn value(&mut self, owner: &str, path: Vec<String>)` — A name used as a value inside `owner`.
- `pub fn type_ref(&mut self, owner: &str, path: Vec<String>)` — A type mentioned inside `owner`.
- `call` — A call made by `owner`. `method` marks `receiver.name(...)` where the receiver's type is unknown; `self_type` is set only when the receiver is the instance itself. `at` is the node of the callee's name; where it starts is the call's place in the source.
- `pub fn import(&mut self, path: Vec<String>, alias: &str, glob: bool, public: bool)` — A name bound by an import. An empty `alias` binds nothing and only states that the file depends on `path`.
- `pub fn import_external(&mut self, package: &str, name: &str)` — the file takes `name` from `package`, which is outside the repository. An empty `name` states only that it uses the package.
- `pub fn inherits(&mut self, qualified: &str, parent: Vec<String>)` — `qualified` extends or implements the type written as `parent`.

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

## See also

- [[_index]]
- [[Lang Module]]
