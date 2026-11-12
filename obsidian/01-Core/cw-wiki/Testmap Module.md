---
tags: [core, cw-wiki, module]
---

# Testmap Module

> Test map: which tests exist, what they exercise, and which to run after a change.

## Source

- `crates/cw-wiki/src/testmap.rs` — module `cw_wiki::testmap`

## How it works

Main items:

- `TestKind` — enum
- `TestFile` — A file that contains tests.
- `TestCase` — struct
- `ImpactedTestFile` — A test file worth running after a change.
- `FileTests` — how one source file is tested.
- `impacted_tests` — tests to run when `changed` files change: tests in the changed files, and tests in every file that depends on them, however many hops away.
- `test_map` — tests per file and the files nothing tests.
- `TestMap` — struct

## Depends on

- [[Index]] — uses `Index`, `crates`, `files`, `symbols`
- [[Workspace Module]] — uses `CrateInfo`, `module_path`, `owning_crate`
- [[Error Module]] — uses `Result`
- [[Gitintel Module]] — uses `dependents_by_file`
- [[Rustscan Module]] — uses `is_test_path`
- [[Scanned]] — uses `new`

## Used by

- [[cw-wiki]] — from `lists_the_tests_a_change_can_break`, `maps_tests_to_the_files_they_reach`
- [[Api Module]] — from `impacted_tests`, `test_map`
- [[codewiki]] — from `run`

## See also

- [[_index]]
- [[cw-wiki]]
