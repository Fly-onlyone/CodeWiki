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

- [[TestKind]] — enum
- `pub struct TestCase` — public struct; used by `CSharpTests::command`, `CTests::command`, `GoTests::command`, `JsTests::command` and 7 more.
- `pub struct TestFile` — A file that contains tests.
- `pub struct FileTests` — how one source file is tested.
- `pub struct TestMap` — public struct; used by `test_map`.
- `struct Env<'a>` — what a language's test code is, found in a repository. Built once for a run from the index and the checkout.
- `trait TestLanguage: Sync` — the two things that differ between languages: what a test is, and the command that runs one. Everything else about the map is reachability over the dependency graph.
- `fn language_of(language: Language) -> Option<&'static dyn TestLanguage>` — private function, returns `Option<&'static dyn TestLanguage>`; called by `gather`.
- `fn is_csharp_test(symbol: &Symbol) -> bool` — A test method: an attribute of a test framework on a method.
- `enum GoTest` — what kind of test a Go function is, by the rules of `go test`: the name starts with `Test`, `Benchmark`, `Fuzz` or `Example` and what follows does not start with a lower-case letter, and the parameter says which: a `*testing.T`, `*testing.B` or `*testing.F`, none for an example.
- `fn is_jvm_test(symbol: &Symbol) -> bool` — A test: a method with one of the annotations (as written: `Test`, `org.junit.Test`), or a class that extends one of Kotest's spec styles, whose tests are written in its constructor and carry no annotation.
- `fn parent_dir(path: &str) -> &str` — private function, returns `&str`; called by `JsTests::command`, `JvmTests::gradle`, `JvmTests::maven`, `JvmTests::module`.
- `fn join_path(dir: &str, name: &str) -> String` — private function, returns `String`; called by `JvmTests::gradle`, `JvmTests::is_gradle`, `JvmTests::maven`, `JvmTests::module` and 1 more.
- `struct JvmTests` — Java and Kotlin tests, run by Gradle or Maven. A file is a test file when it is test code by name or by its Gradle source set (`src/test`, `src/commonTest`, `src/jvmTest`). One command runs a module's tests of both languages, so the two share this entry.
- `fn gather(root: &Path, index: &Index) -> Result<Facts>` — private function, returns `Result<Facts>`; called by `impacted_tests`, `test_map`.
- `distances` — Breadth-first distances from `start` along `next`, `start` itself at 0.
- `pub fn test_map(root: &Path, index: &Index) -> Result<TestMap>` — tests per file and the files nothing tests.
- `pub struct ImpactedTestFile` — A test file worth running after a change.
- `pub struct Impact` — public struct; used by `impacted_tests`.
- `pub fn impacted_tests(root: &Path, index: &Index, changed: &[String]) -> Result<Impact>` — tests to run when `changed` files change: tests in the changed files, and tests in every file that depends on them, however many hops away.

And 21 more items.

Submodules: [[Js Module]], [[Python Module (testmap)]].

## Depends on

- [[Units Module]] — uses `is_csharp_test_attribute`, `is_jvm_test_file`, `is_test_file`, `links_a_test_framework`
- [[Index]] — uses `Index`, `crates`, `files`, `symbols`
- [[C Module]] — uses `TEST_ATTR`, `TEST_FRAMEWORK_ATTR`, `TEST_NAME_ATTR`
- [[Language]] — uses `Language`, `from_path`, `is_c_family`
- [[Cmake Module]] — uses `Target`, `read_cmake`
- [[Workspace Module]] — uses `module_path`, `owning_crate`
- [[JsRepo]] — uses `JsRepo`, `read`
- [[PythonRoots]] — uses `PythonRoots`, `read`

## Used by

- [[cw-wiki]] — from `a_python_project_without_pytest_runs_unittest_by_module_name`, `impact_of`, `lists_the_cpp_tests_a_change_can_break`, `lists_the_csharp_tests_a_change_can_break`
- [[Js Module]] — from `JsTests`, `command`, `commands`, `find_tests`
- [[PythonTests]] — from `PythonTests`, `command`, `commands`, `is_test_file`
- [[Api Module]] — from `impacted_tests`, `test_map`
- [[codewiki]] — from `run`
- [[Python Module (testmap)]] — from `read_project`

## Gotchas

- A test file that itself changed is at distance 0.
- Paths with backslashes are normalised to slashes.
- Only files directly in a `tests/` folder get an integration command. Helper modules deeper in it get an empty command.
- A file with only declarations and re-exports is never called untested, because it has no functions.
- A test file that cannot be read from disk is skipped without an error.

## See also

- [[_index]]
- [[cw-wiki]]
