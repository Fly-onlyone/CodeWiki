---
tags: [core, cw-wiki, module]
---

# Python Module (testmap)

> Python tests: pytest, and unittest where a project names no pytest.

## What it does

A test file is `test_*.py`, `*_test.py`, or any file under a `tests` or `test` folder; `conftest.py` is support and holds no test. A test is a function `test_*`, a method `test_*` of a class `Test*` (pytest) and a method `test_*` of a class that derives from a `unittest.TestCase` (any base whose name ends in `TestCase`, also through a base class of the same file).

The runner is pytest when the project names it: `[tool.pytest.ini_options]` or any `pytest` in its build files, `pytest.ini`, a `[pytest]` section in `tox.ini`, `[tool:pytest]` in `setup.cfg`, a `conftest.py`, or the file importing `pytest`; else `python -m unittest`. The project's own tool runs it when its files say which: `uv run` (`uv.lock`), `poetry run` (`poetry.lock` or `[tool.poetry]`), `pdm run` (`pdm.lock`), `hatch run` (`[tool.hatch.envs`). Commands run from the repository root, or, for a project below it, after `cd <project folder> &&`, with the paths as that folder sees them. A `python -m unittest` name is the module's dotted name as the scan gave it, so the package must be importable from that folder.

## Source

- `crates/cw-wiki/src/testmap/python.rs` — module `cw_wiki::testmap::python`

## How it works

1. `PythonTests::is_test_file` in [[PythonTests]] calls `is_test_path`.
2. `PythonTests::project` in [[PythonTests]] calls `read_project`. `read_project` calls `ancestors` in [[Jsbuild Module]], then `join_path` in [[Testmap Module]], then `word`.
3. `PythonTests::command` in [[PythonTests]] calls `quote`.
4. `PythonTests::name_of` in [[PythonTests]] calls `is_unittest_class`.
5. `PythonTests::commands` in [[PythonTests]] calls `unique`.

4 more entry points: `file_of`, `unittest_module`, `words`, `split_command`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const MAX_NAMES: usize` — more tests than this in one run and the run names the files instead; more files than `MAX_FILES` and it runs the whole project.
- `pub(super) struct PyProject` — what a project says about how its tests run.
- `pub(super) fn is_test_path(path: &str) -> bool` — true for a file of Python test code: `test_*.py`, `*_test.py`, anything under a folder named `tests` or `test`, and `conftest.py` (support).
- `fn word(text: &str, needle: &str) -> bool` — private function, returns `bool`; called by `read_project`.
- `fn read_project(env: &Env, dir: &str) -> PyProject` — what the files at and above `dir` say about the project's tests.
- `fn quote(path: &str) -> String` — A path as one shell argument.
- `fn is_unittest_class(symbols: &[Symbol], class: &Symbol, depth: usize) -> bool` — true when the class `name` of the file's symbols derives from a `unittest.TestCase`, directly or through a class of the same file.
- [[PythonTests]] — struct
- `fn unique(items: Vec<String>) -> Vec<String>` — private function, returns `Vec<String>`; called by `PythonTests::commands`.
- `fn file_of(arg: &str) -> String` — the file of a pytest id, as written (quotes kept).
- `fn unittest_module(arg: &str) -> String` — the module of a unittest id `pkg.mod.Class.method`.
- `pub(super) fn words(command: &str) -> Vec<String>` — the words of a command, a double-quoted word kept whole with its quotes.
- `fn split_command(command: &str) -> Option<(String, bool, Vec<String>)>` — A command written by `PythonTests::command` as its head (everything up to and including the runner), whether the runner is unittest, and its arguments.
- `pub(super) type Projects = HashMap<String, PyProject>` — what `Env::py_projects` keeps: a project folder and what it says.

And 1 more private item with no comment.

## Depends on

- [[Testmap Module]] — uses `Env`, `join_path`
- [[Jsbuild Module]] — uses `ancestors`
- [[Symbol]] — uses `Symbol`

## Used by

- [[PythonTests]] — from `command`, `commands`, `is_test_file`, `name_of`
- [[Testmap Module]] — from `Env`
- [[Js Module]] — from `split_files`

## See also

- [[_index]]
- [[cw-wiki]]
