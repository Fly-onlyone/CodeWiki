---
tags: [core, cw-parse]
---

# Sink

> Collects the facts of one file.

## What it does

`Sink` collects the facts of one source file while an extractor walks its syntax tree. The Python, TypeScript and Java extractors all call the same methods on it, so they all produce the same `ParsedFile`: symbols, calls, value and type references, imports and inheritance. This keeps the language-specific code small, because it only has to decide what a node means.

## Source

- `crates/cw-parse/src/lang.rs` — definition of `Sink`

## How it works

An extractor makes a `Sink` with `new`, giving the file, its text, the module name and whether the tree had syntax errors. It then calls `module_symbol` once, `symbol` for each declaration, and `value`, `type_ref`, `call`, `import`, `import_external` and `inherits` for what it finds. At the end it takes `sink.out`.

`symbol` pushes a public symbol with the qualified name `scope::name` and returns it. The caller then fills in the signature, doc, visibility and attributes. `call` stores where the callee's name starts, so a later step can order calls as they stand in the source. `inherits` becomes an impl fact that links a type to its parent.

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

## Gotchas

- `value` and `type_ref` remember what they saw for each owner. The same path for the same owner is stored once.
- `type_ref` ignores single uppercase letters such as `T`, which are almost always generic parameters.
- `inherits` only accepts a name inside the file's own module. A name outside it is silently skipped.
- `import` with an empty path does nothing. An empty alias is allowed and only states that the file depends on the path.
- `symbol` returns a mutable reference into a vector. Fill it before you call another `Sink` method.

## See also

- [[_index]]
- [[Lang Module]]
