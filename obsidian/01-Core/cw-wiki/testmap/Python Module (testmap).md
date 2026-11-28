---
tags: [core, cw-wiki, module]
---

# Python Module (testmap)

> Python tests: pytest, and unittest where a project names no pytest.

## What it does

Teaches the test map about Python tests. It runs pytest when the project names it, and `python -m unittest` otherwise.

A test file is `test_*.py`, `*_test.py`, or any file under a `tests` or `test` folder. `conftest.py` is support and holds no test. A test is a function `test_*`, a method `test_*` of a class `Test*`, or a method `test_*` of a class that derives from a `unittest.TestCase` (any base whose name ends in `TestCase`, also through a base class of the same file).

The project's own tool wraps the command when its files say which: `uv run`, `poetry run`, `pdm run` or `hatch run`.

## Source

- `crates/cw-wiki/src/testmap/python.rs` — module `cw_wiki::testmap::python`

## How it works

`read_project` walks from the project folder up through its ancestors. It marks the project as pytest when it finds `pytest.ini`, a `conftest.py`, a `[pytest]` section in `tox.ini`, `[tool:pytest]` in `setup.cfg`, `[tool.pytest` in `pyproject.toml`, or the whole word `pytest` in build or requirements files (`word`). It picks a command prefix from `uv.lock`, `poetry.lock`, `pdm.lock` or hatch environments. The nearest prefix wins. Results are kept per folder in `Env::py_projects`.

[[PythonTests]] builds the command. For pytest it uses ids like `path::Class::test`. Past 8 ids it names the file. For unittest it uses the dotted module name from the scan, `pkg.mod.Class.method`.

`is_unittest_class` reads the bases from the class signature and follows a base of the same file up to 4 levels. `split_command` and `words` read a command back into a head and arguments, so many files can be merged into one run. `file_of` and `unittest_module` turn an id back into its file or module.

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

## Gotchas

- Past 8 names a run names the files. Past 8 files it runs the whole project.
- A unittest name is the module's dotted name as the scan gave it, so the package must be importable from the project folder.
- Unittest runs only methods of `TestCase` classes. A plain `Test*` class counts for pytest, not for unittest.
- A file that imports `pytest` is run with pytest even when the project names none.
- A project below the root gets `cd <folder> &&` in front, and paths as that folder sees them.

## See also

- [[_index]]
- [[cw-wiki]]
