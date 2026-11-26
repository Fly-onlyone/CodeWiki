---
tags: [core, cw-wiki, module]
---

# Testmap Module

> Test map: which tests exist, what they exercise, and which to run after a change.

## What it does

Answers two questions about Rust tests. First: which tests exist, which source files do they reach, and which files has no test reached? Second: after these files change, which tests should I run, and with which command? It works without coverage data. A test reaches a file when it can get there through the file dependency graph. That means can reach, not does execute.

## Source

- `crates/cw-wiki/src/testmap.rs` — module `cw_wiki::testmap`

## How it works

`gather` reads every indexed Rust file from disk and scans it with [[Scanned]] to find test functions. The index cannot be used for this, because the structural parser leaves unit-test modules out. Files under `tests`, `benches` or `examples` count as integration tests. Other files with tests count as unit tests. For each test file `command` derives a `cargo test` command from its crate: `--test name` for a file directly in `tests/`, `--bins` for a binary, and `--lib` plus a module path for a library module.

`distances` is a breadth-first walk over the map from a file to the files that depend on it. It returns the number of hops from the start.

`test_map` walks from each source file. A test file found at one hop is direct. At two or more hops it is indirect. A source file that defines functions, has no unit tests and is reached by no test file goes into `untested`.

`impacted_tests` walks from each changed file and keeps every test file it reaches, with the smallest distance and the changed files that cause it. The result is sorted nearest first, with distinct commands. Changed source files that reach no test are listed as untested changes.

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

## Gotchas

- A test file that itself changed is at distance 0.
- Paths with backslashes are normalised to slashes.
- Only files directly in a `tests/` folder get an integration command. Helper modules deeper in it get an empty command.
- A file with only declarations and re-exports is never called untested, because it has no functions.
- A test file that cannot be read from disk is skipped without an error.

## See also

- [[_index]]
- [[cw-wiki]]
