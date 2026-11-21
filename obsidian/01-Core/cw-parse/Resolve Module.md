---
tags: [core, cw-parse, module]
---

# Resolve Module

> Name resolution without a compiler.

## What it does

Turns the per-file facts into graph edges. Resolution is heuristic, so each edge records how it was matched; see `cw_core::Confidence`.

## Source

- `crates/cw-parse/src/resolve.rs` — module `cw_parse::resolve`

## How it works

1. `Resolver::resolve` in [[Resolver]] calls `leaves`.
2. `Resolver::by_bare_name` in [[Resolver]] calls `home_module`.
3. `Resolver::by_bare_name` in [[Resolver]] calls `visible_from`. `visible_from` calls `home_module`, then `is_separate_target`, then `within`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

## See also

- [[_index]]
- [[cw-parse]]
