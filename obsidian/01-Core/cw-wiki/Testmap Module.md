---
tags: [core, cw-wiki, module]
---

# Testmap Module

> Test map: which tests exist, what they exercise, and which to run after a change.

## What it does

Tests are found with a lexical scan (`crate::rustscan`) because the structural parser leaves unit-test modules out of the index. What a test exercises is read from the file dependency graph: a test file exercises the files it depends on, directly or through further hops. No coverage data is involved, so "exercises" means "can reach", not "executes".

## Source

- `crates/cw-wiki/src/testmap.rs` — module `cw_wiki::testmap`

## How it works

1. `run` in [[codewiki]] and `test_map` in [[Api Module]] call `test_map`. `test_map` calls `gather`, then `distances`. `gather` calls `is_test_path` in [[Rustscan Module]], then `Scanned::new` in [[Scanned]], then `command`, then `dependents_by_file` in [[Gitintel Module]].
2. `run` in [[codewiki]] and `impacted_tests` in [[Api Module]] call `impacted_tests`. `impacted_tests` calls `gather`, then `distances`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub enum TestKind` — public enum; used by `ImpactedTestFile`, `TestFile`, `command`.
- `pub struct TestCase` — public struct; used by `TestFile`, `gather`.
- `pub struct TestFile` — A file that contains tests.
- `pub struct FileTests` — how one source file is tested.
- `pub struct TestMap` — public struct; used by `test_map`.
- `fn command(crates: &[CrateInfo], path: &str, kind: TestKind) -> String` — private function, returns `String`; called by `gather`.
- `struct Facts` — everything the analyses below need, gathered once.
- `fn gather(root: &Path, index: &Index) -> Result<Facts>` — private function, returns `Result<Facts>`; called by `impacted_tests`, `test_map`.
- `distances` — Breadth-first distances from `start` along `next`, `start` itself at 0.
- `pub fn test_map(root: &Path, index: &Index) -> Result<TestMap>` — tests per file and the files nothing tests.
- `pub struct ImpactedTestFile` — A test file worth running after a change.
- `pub struct Impact` — public struct; used by `impacted_tests`.
- `pub fn impacted_tests(root: &Path, index: &Index, changed: &[String]) -> Result<Impact>` — tests to run when `changed` files change: tests in the changed files, and tests in every file that depends on them, however many hops away.

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
