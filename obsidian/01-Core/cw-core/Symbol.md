---
tags: [core, cw-core]
---

# Symbol

> A named code entity.

## What it does

One named code entity found by the parser: a function, method, type, module, constant and so on. `qualified` is unique inside a repository, for example `cw_core::hash::sha256_hex`, and is the key every edge uses. Notes, health checks, the resolver and queries all start from a `Symbol`.

## Source

- `crates/cw-core/src/model.rs` — definition of `Symbol`

## How it works

A symbol records its short `name`, its [[SymbolKind]] and its visibility (public, crate or private), the `file`, and a 1-based inclusive line range. `signature` is the declaration text up to the body with whitespace collapsed. `doc` is the leading documentation comment with markers removed. `parent` is the qualified name of the enclosing module, impl or trait, which gives the tree. `attrs` lists attribute names such as `derive`, `test` or `tokio::main`.

The [[Index]] stores symbols and returns them by name, file or qualified name. The [[Resolver]] looks symbols up to attach edges.

## Items

Declared as `pub struct Symbol`.

## Depends on

- [[SymbolKind]] — uses `SymbolKind`
- [[Visibility]] — uses `Visibility`

## Used by

- [[Resolver]] — from `Resolver`, `by_bare_name`, `c_callee`, `c_home`
- [[Explain Module]] — from `Callers`, `Step`, `Walk`, `callers_sentence`
- [[Testmap Module]] — from `Env`, `command`, `ctest_alternative`, `framework`
- [[Health Module]] — from `dead_code`, `file_health`, `is_c_api`, `is_csharp_main`
- [[Repo]] — from `Repo`, `build`, `children_of`, `is_go_test_package`
- [[Units Module]] — from `discover`, `is_c_entry`, `is_csharp_entry`, `is_go_entry`
- [[Scan Module]] — from `RepoFacts`, `merge_package`, `merge_partial`, `scan_repo`
- [[Plan Module]] — from `enclosing_module`, `files_of`, `module_title`, `plan`

## Gotchas

- Do not treat `name` as unique. Only `qualified` is.
- `attrs` matters for dead-code checks. An attribute macro can make an item reachable with no visible caller, so a symbol with attributes is only a possible dead one.
- `signature` and `doc` are text copies, taken when the file was parsed.

## See also

- [[_index]]
- [[Model Module]]
