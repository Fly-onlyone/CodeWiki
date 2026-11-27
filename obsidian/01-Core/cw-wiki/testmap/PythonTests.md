---
tags: [core, cw-wiki]
---

# PythonTests

> `PythonTests` struct defined in `cw_wiki::testmap::python` with 8 methods.

## Source

- `crates/cw-wiki/src/testmap/python.rs` — definition of `PythonTests`

## How it works

What the functions of this type call:

1. `is_test_file` calls `is_test_path` in [[Python Module (testmap)]].
2. `command` calls `project_dir`, then `project`, then `quote` in [[Python Module (testmap)]]. `project` calls `read_project` in [[Python Module (testmap)]].
3. `commands` calls `split_command`, then `unique`, then `unittest_module`, then `file_of` (all in [[Python Module (testmap)]]).

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

## See also

- [[_index]]
- [[Python Module (testmap)]]
