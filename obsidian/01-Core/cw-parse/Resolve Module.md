---
tags: [core, cw-parse, module]
---

# Resolve Module

> Name resolution without a compiler.

## Source

- `crates/cw-parse/src/resolve.rs` — module `cw_parse::resolve`

## How it works

Main items:

- [[Resolver]] — struct
- `ExternalUse` — A `use` of a crate that is not part of the repository.
- `Resolved` — struct
- `MAX_REEXPORT_DEPTH` — const
- `home_module` — the module a Rust symbol is declared in: its qualified name without its own name, and without the type for a method (`module::Type::name`).
- `is_separate_target` — A file of an integration test, a bench or an example: a crate of its own, whose items the library cannot name.
- `within` — `module` is `outer` or nested inside it.
- `visible_from` — whether Rust code in `module` can name `symbol`: a private item in its own module and the ones inside it, a `pub(crate)` item in its crate, and an item of a test, bench or example only from that file.

## Depends on

- [[Model Module]] — uses `Edge`
- [[Symbol]] — uses `Symbol`

## Used by

- [[Resolver]] — from `by_bare_name`, `resolve`, `resolve_abs`
- [[Scan Module]] — from `RepoFacts`
- [[Index]] — from `external_uses`

## See also

- [[_index]]
- [[cw-parse]]
