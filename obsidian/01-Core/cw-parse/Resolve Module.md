---
tags: [core, cw-parse, module]
---

# Resolve Module

> Name resolution without a compiler.

## What it does

This module is name resolution without a compiler. It takes the per-file facts and produces the graph edges, and it lists the outside packages each file uses. The work is done by the [[Resolver]]. Because the matching is heuristic, each edge records how sure the match was, as a confidence level.

## Source

- `crates/cw-parse/src/resolve.rs` — module `cw_parse::resolve`

## How it works

`Resolved` is the result: a list of edges and a list of `ExternalUse` rows. An `ExternalUse` says that a file uses a package outside the repository, and which names it takes from it. `leaves` trims that list so a module is left out when a name inside it is listed, for example `de` beside `de::Visitor`.

The rest are helpers for Rust visibility. `home_module` gives the module a symbol lives in, and for a method it skips the type too. `within` tests nesting. `visible_from` decides whether code in a module can name a symbol: private items only inside their module, `pub(crate)` items only inside their crate. `is_separate_target` marks files under `tests`, `benches` or `examples` outside `src`, which behave as crates of their own.

## Items

- `pub struct ExternalUse` — A `use` of a crate that is not part of the repository.
- `fn leaves(names: BTreeSet<String>) -> Vec<String>` — `names` without each one that only leads to another: `de` beside `de::Visitor`.
- `pub struct Resolved` — public struct; used by `Resolver::resolve`.
- `const TOOLCHAIN_CRATES: [&str; 5]` — crates that ship with the toolchain; never reported as integrations.
- `fn home_module(symbol: &Symbol) -> &str` — the module a Rust symbol is declared in: its qualified name without its own name, and without the type for a method (`module::Type::name`).
- `fn within(module: &str, outer: &str) -> bool` — `module` is `outer` or nested inside it.
- `fn is_separate_target(file: &str) -> bool` — A file of an integration test, a bench or an example: a crate of its own, whose items the library cannot name.
- `fn visible_from(symbol: &Symbol, module: &str) -> bool` — whether Rust code in `module` can name `symbol`: a private item in its own module and the ones inside it, a `pub(crate)` item in its crate, and an item of a test, bench or example only from that file.
- `const COMMON_METHODS: [&str; 64]` — method names so common in the standard library that a receiver of unknown type almost never means a repository method of the same name.
- [[Resolver]] — struct

And 1 more private item with no comment.

## Depends on

- [[Model Module]] — uses `Edge`
- [[Symbol]] — uses `Symbol`

## Used by

- [[Resolver]] — from `by_bare_name`, `resolve`, `resolve_abs`
- [[Scan Module]] — from `RepoFacts`
- [[Index]] — from `external_uses`
- [[RenderCtx]] — from `render_integration`

## Gotchas

- `COMMON_METHODS` lists 64 standard library names such as `len`, `get` and `push`. A call on an unknown receiver with one of these names is almost never a repository method, so only a private method of the caller's own module can match.
- `TOOLCHAIN_CRATES` keeps `std`, `core`, `alloc`, `proc_macro` and `test` out of the outside-package report.
- `MAX_REEXPORT_DEPTH` is 6. A chain of re-exports longer than that is not followed, and the name stays unresolved.
- `ExternalUse` names are in name order and use the language's own style: `extract::Path` for Rust, `adapters.HTTPAdapter` for Python and Java.

## See also

- [[_index]]
- [[cw-parse]]
