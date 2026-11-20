---
tags: [core, cw-parse]
---

# Resolver

> `Resolver` struct defined in `cw_parse::resolve` with 9 methods.

## Source

- `crates/cw-parse/src/resolve.rs` — definition of `Resolver`

## How it works

1. `scan_repo` in [[Scan Module]] calls `new`. `new` calls `absolutize`.

## Items

Declared as `pub struct Resolver<'a>`.

Members:

- `new` — public method, returns `Self`; called by `scan_repo`.
- `absolutize` — rewrite `crate`, `self`, `super` and `Self` prefixes to absolute form. Paths with no such prefix are returned unchanged (still relative).
- `fn resolve_abs(&self, segs: &[String], depth: usize) -> Option<&'a Symbol>` — resolve an absolute path, following re-exports through `use` aliases and glob imports of the modules along the way.
- `lookup` — resolve a path as written at a use site inside `module`.
- `lookup_from` — `Self::lookup`, then the scopes enclosing `from`: an item declared inside a function is visible to the code around it.
- `pub fn with_crates(mut self, crates: &'a [CrateInfo]) -> Self` — tell the resolver which crates each Cargo package depends on, so that a name never matches a method in a crate the caller cannot reach.
- `fn reaches(&self, module: &str, symbol: &Symbol) -> bool` — whether code in `module` can reach the crate `symbol` is declared in: its own crate, or one its manifest depends on. A crate no manifest describes reaches every crate.
- `by_bare_name` — fall back to matching by bare name when path resolution fails. A name that only one method has is taken to mean it, unless the caller cannot see that method: a call such as `request.header(..)` on a type from a dependency must not bind to a private `header` elsewhere, and gpui's `div().size(..)` must not bind to a `size` in a crate the caller's crate does not depend on. When several methods share the name, a Rust caller gets the one declared in its own module, if there is just one. With `private_here`, the name is common enough that only a private method of the caller's own module is taken.
- `pub fn resolve(&self, files: &[(String, ParsedFile)]) -> Resolved` — build every edge for the repository.

## Depends on

- [[Resolve Module]] — uses `COMMON_METHODS`, `ExternalUse`, `MAX_REEXPORT_DEPTH`, `Resolved`
- [[Model Module]] — uses `Edge`, `EdgeKind`, `file_node`
- [[Language]] — uses `Language`, `from_path`
- [[SymbolKind]] — uses `SymbolKind`, `is_type`
- [[Workspace Module]] — uses `CrateInfo`
- [[Confidence]] — uses `Confidence`
- [[Symbol]] — uses `Symbol`
- [[ParsedFile]] — uses `ParsedFile`

## Used by

- [[Scan Module]] — from `scan_repo`

## See also

- [[_index]]
- [[Resolve Module]]
