---
tags: [core, cw-parse, module]
---

# Pyroots Module

> Where Python's import names start: the folders an absolute import is taken from, read from the build files (`pyproject.toml`, `setup.cfg`, `setup.py`) and never run.

## What it does

A folder of `.py` files below a root is part of the dotted path whether or not it has an `__init__.py`: that is PEP 420, the namespace package. The roots of a project are the folder of its build file, a `src` folder under it, and what the build file declares:

- `[tool.setuptools.packages.find] where = [...]` and `[tool.setuptools.package-dir]` (`"" = "src"`, or a package named for a folder: `shop = "lib/shop_impl"`),
- `[options] package_dir = =src` and `[options.packages.find] where` of `setup.cfg`,
- the literals `package_dir={"": "src"}` and `where="src"` of `setup.py`,
- `[tool.poetry] packages = [{ include, from }]`,
- `[tool.hatch.build.targets.wheel] packages` and `sources`,
- `[tool.pdm.build] package-dir`, and `[tool.maturin] python-source`.

A file that no declared root holds keeps the rule it had before: the name starts at the outermost folder with an `__init__.py`; with none, at the project's folder, or at the repository root (minus a leading `src` or `lib`) when no build file is above it.

## Source

- `crates/cw-parse/src/pyroots.rs` — module `cw_parse::pyroots`

## How it works

1. `discover` in [[Units Module]] calls `project_name`. `project_name` calls `get`.
2. `PythonRoots::read` in [[PythonRoots]] calls `pyproject_roots`. `pyproject_roots` calls `strings`, then `get`, then `root`. `root` calls `join_norm` in [[Jsbuild Module]].
3. `PythonRoots::read` in [[PythonRoots]] calls `setup_cfg_roots`. `setup_cfg_roots` calls `root`.
4. `PythonRoots::read` in [[PythonRoots]] calls `setup_py_roots`. `setup_py_roots` calls `root`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const BUILD_FILES: [&str; 3]` — build files that make a folder a Python project.
- `struct SourceRoot` — A folder that import names start at.
- [[PythonRoots]] — the Python projects of a repository and their import roots.
- `pub fn project_name(text: &str) -> Option<String>` — the name a `pyproject.toml` gives its project: `[project] name`, else `[tool.poetry] name`.
- `fn strings(value: Option<&toml::Value>) -> Vec<String>` — A TOML value that is a string or a list of strings.
- `fn root(dir: &str, below: &str) -> Option<SourceRoot>` — private function, returns `Option<SourceRoot>`; called by `PythonRoots::module`, `PythonRoots::read`, `pyproject_roots`, `setup_cfg_roots` and 1 more.
- `fn get<'a>(value: &'a toml::Value, path: &[&str]) -> Option<&'a toml::Value>` — private function, returns `Option<&'a toml::Value>`; called by `project_name`, `pyproject_roots`.
- `fn pyproject_roots(dir: &str, text: &str) -> Vec<SourceRoot>` — private function, returns `Vec<SourceRoot>`; called by `PythonRoots::read`.
- `fn setup_cfg_roots(dir: &str, text: &str) -> Vec<SourceRoot>` — `package_dir` and `[options.packages.find] where` of `setup.cfg`.
- `fn setup_py_roots(dir: &str, text: &str) -> Vec<SourceRoot>` — the literals `package_dir={"": "src"}` and `where="src"` of a `setup.py`. A path computed in code is not followed.

## Depends on

- [[Jsbuild Module]] — uses `join_norm`

## Used by

- [[PythonRoots]] — from `PythonRoots`, `module`, `read`
- [[Units Module]] — from `discover`

## See also

- [[_index]]
- [[cw-parse]]
