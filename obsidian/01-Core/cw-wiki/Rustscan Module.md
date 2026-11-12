---
tags: [core, cw-wiki, module]
---

# Rustscan Module

> A light lexical scan of Rust source, shared by the test map and the security rules.

## Source

- `crates/cw-wiki/src/rustscan.rs` — module `cw_wiki::rustscan`

## How it works

Main items:

- [[Scanned]] — struct
- [[Literal]] — one string literal.
- [[TestFn]] — A `test`-like function found in a file.
- `is_ident` — function
- `matching` — offset one past the bracket that closes the one at `open`.
- `blank` — function
- `is_test_path` — true for paths under a `tests`, `benches` or `examples` directory.
- `find_tests` — find test functions and test-only byte ranges in masked code.

## Depends on

- [[Literal]] — uses `Literal`
- [[TestFn]] — uses `TestFn`

## Used by

- [[Refactor Module]] — from `break_cycles`, `extract_types`, `move_functions`, `split_files`
- [[Scanned]] — from `find`, `matching`, `new`, `statement_end`
- [[Contracts Module]] — from `scan`, `scan_source`
- [[Security Module]] — from `scan`, `scan_source`
- [[Coverage Module]] — from `report`
- [[Testmap Module]] — from `gather`

## See also

- [[_index]]
- [[cw-wiki]]
