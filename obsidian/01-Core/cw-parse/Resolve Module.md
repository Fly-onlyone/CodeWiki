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
- `COMMON_METHODS` — method names so common in the standard library that a receiver of unknown type almost never means a repository method of the same name.
- `TOOLCHAIN_CRATES` — crates that ship with the toolchain; never reported as integrations.

## Depends on

- [[Model Module]] — uses `Edge`

## Used by

- [[Resolver]] — from `resolve`, `resolve_abs`
- [[Scan Module]] — from `RepoFacts`
- [[Index]] — from `external_uses`

## See also

- [[_index]]
- [[cw-parse]]
