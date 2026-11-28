---
tags: [core, cw-wiki]
---

# PythonTests

> `PythonTests` struct defined in `cw_wiki::testmap::python` with 8 methods.

## What it does

The Python entry in the test map's list of languages. It fills in the two things that differ between languages: what a test is, and the command that runs it. Details of the rules are in [[Python Module (testmap)]].

## Source

- `crates/cw-wiki/src/testmap/python.rs` — definition of `PythonTests`

## How it works

`is_test_file` calls `is_test_path`. `tests` asks the scanned symbols of the file for functions and methods that `name_of` accepts, such as `test_a` or `TestC::test_m`, and skips `conftest.py`. `command` finds the owning project with `project_dir` (through [[PythonRoots]]), reads its settings once with `project`, and writes either a pytest line or a unittest line. `commands` merges the commands of all impacted files by their head, up to and including the runner. Past 8 names it names files. Past 8 files it runs the whole project. A file named whole swallows the ids inside it.

## Items

Declared as `pub(super) struct PythonTests`.

Members:

- `fn project_dir<'e>(env: &'e Env, path: &str) -> &'e str` — the folder of the project that owns `path`; empty for the repository.
- `fn project(env: &Env, dir: &str) -> PyProject` — private method, returns `PyProject`; called by `PythonTests::command`.
- `fn name_of(symbols: &[Symbol], symbol: &Symbol) -> Option<String>` — `Class::method` or `function`, the name of a test as pytest gives it.
- `fn language(&self) -> Language` — public method, returns `Language`.
- `fn is_test_file(&self, _: &Env, path: &str) -> bool` — public method, returns `bool`.
- `fn command(&self, env: &Env, path: &str, _: TestKind, tests: &[TestCase]) -> String` — `pytest path/test_x.py::test_a path/test_x.py::TestC::test_m`, or `python -m unittest pkg.test_x.TestC.test_m`.
- `fn commands(&self, files: &[&ImpactedTestFile]) -> Vec<String>` — one run for each project and runner, over the tests of every impacted file in it: past `MAX_NAMES` names the files, past `MAX_FILES` files the whole project.

## Depends on

- [[Python Module (testmap)]] — uses `MAX_FILES`, `MAX_NAMES`, `PyProject`, `file_of`
- [[Testmap Module]] — uses `Env`, `ImpactedTestFile`, `TestCase`, `TestLanguage`
- [[Language]] — uses `Language`
- [[Symbol]] — uses `Symbol`
- [[PythonRoots]] — uses `project_of`
- [[TestKind]] — uses `TestKind`

## Used by

- [[Testmap Module]] — from `LANGUAGES`

## Gotchas

- `command` gives an empty string for a unittest file when the scan found no module symbol.
- Project settings are cached by folder in `Env::py_projects`. A run of the map sees one answer for each folder.

## See also

- [[_index]]
- [[Python Module (testmap)]]
