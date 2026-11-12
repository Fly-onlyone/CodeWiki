---
tags: [core, cw-wiki, module]
---

# Units Module

> Units: the top-level parts of a repository, each with its own folder under `01-Core`.

## Source

- `crates/cw-wiki/src/units.rs` — module `cw_wiki::units`

## How it works

Main items:

- [[Unit]] — one top-level part of the repository.
- [[Units]] — every unit of a repository and the lookup from code to unit.
- `Entry` — A symbol where execution starts.
- `Manifest` — A build or dependency file that belongs to a unit.
- `is_test_file` — true for a test file of a language other than Rust.
- `discover` — find the units of a repository.
- `Family` — language family: one unit never mixes two of these.
- `join` — function

## Depends on

- [[Language]] — uses `Language`, `from_path`
- [[Python Module]] — uses `MAIN_GUARD_ATTR`
- [[Scan Module]] — uses `RepoFacts`
- [[Symbol]] — uses `Symbol`
- [[Unit]] — uses `Unit`
- [[Units]] — uses `Units`

## Used by

- [[Repo]] — from `at`, `is_test`, `new`
- [[RenderCtx]] — from `unit_commands`
- [[Unit]] — from `Unit`

## See also

- [[_index]]
- [[cw-wiki]]
