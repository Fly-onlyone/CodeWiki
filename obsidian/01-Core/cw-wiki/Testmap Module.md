---
tags: [core, cw-wiki, module]
---

# Testmap Module

> Test map: which tests exist, what they exercise, and which to run after a change.

## What it does

Answers two questions about tests, in seven languages: Rust, C#, Go, Java and Kotlin, C and C++, Python, TypeScript and JavaScript. First: which tests exist, which source files do they reach, and which files has no test reached? Second: after these files change, which tests should I run, and with which command? It works without coverage data. A test reaches a file when it can get there through the file dependency graph. That means can reach, not does execute.

## Source

- `crates/cw-wiki/src/testmap.rs` — module `cw_wiki::testmap`

## How it works

Reachability is the same for every language. Two things differ, and a `TestLanguage` entry holds both: what a test is, and the command that runs it. The static `LANGUAGES` list holds one entry per language. Adding a language means adding an entry there.

`gather` builds an `Env` once from the index and the checkout: crates, symbols by file, C# projects, Go modules, CMake targets, JavaScript and Python project facts. For each indexed file whose language is known, it asks the entry whether the file is test code, then which tests it holds. Files kept apart from the code (a `tests` folder, a test project, `_test.go`) count as integration tests. Other files with tests count as unit tests. The entry then derives a command.

How each language finds tests: Rust scans the text lexically, because the structural parser leaves unit-test modules out of the index. C#, Go, Java and Kotlin, C and C++, and Python read the symbols and attributes the parser recorded. TypeScript and JavaScript read the titled calls of `test`, `it` and `describe` from the text.

`gather` also adds edges the import graph lacks: `add_go_package_dependents` makes every non-test file of a Go folder a dependency of that folder's test files, and `add_c_pair_dependents` merges a header and its source into one module.

`distances` is a breadth-first walk from a file to the files that depend on it. `test_map` walks from each source file: a test file one hop away is direct, farther is indirect. A file with functions, no unit tests and no test reaching it goes into `untested`. `impacted_tests` walks from each changed file, keeps each test file with its smallest distance, sorts nearest first, and asks each language to fold its files into distinct commands.

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
- Rust: only files directly in a `tests/` folder get an integration command. Helper modules deeper in it get an empty command.
- Go, Java and Kotlin, and C and C++ fold the names of several tests into one filter. Past 8 names (`MAX_GO_NAMES`, `MAX_JVM_CLASSES`, `MAX_C_TESTS`) the run is the whole package, module or executable, because a long list would not save time.
- C#: `--filter` is added only for VSTest projects. A project on Microsoft.Testing.Platform runs whole, since its filter syntax differs.
- Go benchmarks go to `-bench`. A file of benchmarks alone runs with `-run '^$'`, so it runs no test.
- A file with only declarations and re-exports is never called untested, because it has no functions.
- A test file that cannot be read from disk is skipped without an error.

## See also

- [[_index]]
- [[cw-wiki]]
