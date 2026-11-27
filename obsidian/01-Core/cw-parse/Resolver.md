---
tags: [core, cw-parse]
---

# Resolver

> `Resolver` struct defined in `cw_parse::resolve` with 17 methods.

## What it does

The `Resolver` turns the loose facts of all files into the edges of the code graph (calls, imports, implements, references, declares). The scanner only records names as they were written, such as a call to `shout`. The resolver decides which symbol that name means, with no compiler, using the imports, modules and symbols it knows. It also lists which outside packages each file uses.

## Source

- `crates/cw-parse/src/resolve.rs` — definition of `Resolver`

## How it works

`new` indexes every symbol by qualified name and by bare name. It reads every import and stores it as module, alias, absolute path. Globs are stored apart. `with_crates` adds the dependency list of each Cargo package.

`lookup` resolves a path written inside a module. It builds candidates in order: through an import, relative to the module, as a crate name, then through glob imports. `resolve_abs` tries each, following re-exports through imports and globs up to six hops (`MAX_REEXPORT_DEPTH`). `lookup_from` also checks the scopes around the caller, so an item declared inside a function is found.

`resolve` goes file by file. An import that resolves gives a file-to-file Imports edge. One that does not, and names no local module, crate or toolchain crate, becomes an outside use. Module declarations give Declares edges, and impls of a trait give Implements. A plain call that resolves to a function or method gives a High confidence edge. A method call on a known `self` type is Medium. When the type is unknown, `by_bare_name` matches by name alone and gives Low. Value and type mentions give References.

## Items

Declared as `pub struct Resolver<'a>`.

Members:

- `new` — public method, returns `Self`; called by `scan_repo`.
- `fn see_c_modules(&mut self, symbols: &[Symbol], files: &[(String, ParsedFile)])` — find the modules of C and C++ code, what each includes, and the namespaces that were declared.
- `fn c_seen(&self, module: &str) -> Rc<HashMap<String, u8>>` — the modules `module` sees, nearest first: itself, what its files include, and what those include. Transitive, as the preprocessor is.
- `fn c_home<'s>(&self, symbol: &'s Symbol) -> Option<&'s str>` — the module a C or C++ symbol is declared in.
- `fn c_sees(&self, module: &str, symbol: &Symbol) -> bool` — whether code in `module` can name `symbol`: it is in the module or in a header the module includes, and not private to another file.
- `lookup_c` — resolve a name written in a C or C++ module. The enclosing scopes of `from` come first (the class, then the module), then whatever the module sees, the nearest header first. A name that two headers at the same distance both declare names nothing.
- `fn c_callee(&self, symbol: &'a Symbol) -> Option<&'a Symbol>` — the function a call of `symbol` runs: a constructor for a type, the function or macro itself.
- `fn see_enclosing_namespaces(&mut self)` — C# code sees the types of the namespaces around its own without a `using`: in `A.B.C`, the types of `A.B` and `A`. Each namespace of a C# file opens its ancestors as glob imports, nearest first. These are no import of any file, so they give no import edge.
- `absolutize` — rewrite `crate`, `self`, `super` and `Self` prefixes to absolute form. Paths with no such prefix are returned unchanged (still relative).
- `fn resolve_abs(&self, segs: &[String], depth: usize) -> Option<&'a Symbol>` — resolve an absolute path, following re-exports through `use` aliases and glob imports of the modules along the way.
- `lookup` — resolve a path as written at a use site inside `module`.
- `fn lookup_csharp(&self, path: &[String], module: &str) -> Option<&'a Symbol>` — `Self::lookup` for code in a C# namespace. A name there is a symbol at the place its alias, its namespace, a namespace around it or a `using` gives, so each candidate is one direct lookup.
- `lookup_from` — `Self::lookup`, then the scopes enclosing `from`: an item declared inside a function is visible to the code around it.
- `pub fn with_crates(mut self, crates: &'a [CrateInfo]) -> Self` — tell the resolver which crates each Cargo package depends on, so that a name never matches a method in a crate the caller cannot reach.
- `fn reaches(&self, module: &str, symbol: &Symbol) -> bool` — whether code in `module` can reach the crate `symbol` is declared in: its own crate, or one its manifest depends on. A crate no manifest describes reaches every crate.
- `by_bare_name` — fall back to matching by bare name when path resolution fails. A name that only one method has is taken to mean it, unless the caller cannot see that method: a call such as `request.header(..)` on a type from a dependency must not bind to a private `header` elsewhere, and gpui's `div().size(..)` must not bind to a `size` in a crate the caller's crate does not depend on. When several methods share the name, a Rust caller gets the one declared in its own module, if there is just one. With `private_here`, the name is common enough that only a private method of the caller's own module is taken.
- `pub fn resolve(&self, files: &[(String, ParsedFile)]) -> Resolved` — build every edge for the repository.

## Depends on

- [[Resolve Module]] — uses `COMMON_CSHARP_METHODS`, `COMMON_C_METHODS`, `COMMON_GO_METHODS`, `COMMON_KOTLIN_METHODS`
- [[Model Module]] — uses `Edge`, `EdgeKind`, `file_node`
- [[Language]] — uses `Language`, `from_path`, `is_c_family`
- [[SymbolKind]] — uses `SymbolKind`, `is_type`
- [[C Module]] — uses `NAMESPACE_ATTR`
- [[Kotlin Module]] — uses `EXTENSION_ATTR`
- [[Confidence]] — uses `Confidence`
- [[Symbol]] — uses `Symbol`

## Used by

- [[Scan Module]] — from `scan_repo`

## Gotchas

- Resolution is a guess without a compiler, so each edge carries a confidence of High, Medium or Low.
- A path that does not resolve is never guessed for a plain call. It is assumed to name something outside the repository.
- A name never matches a symbol in another language.
- Bare-name matching for Rust respects visibility and crate dependencies. A crate the caller does not depend on cannot be matched, and common names such as `len` or `get` match only a private method in the caller's own module.
- Tests, benches and examples count as separate crates. Their items are visible only from the same file.
- Edges are de-duplicated. The `site` kept is the earliest call place.
- A caller equal to its target is dropped, so recursion gives no edge.

## See also

- [[_index]]
- [[Resolve Module]]
