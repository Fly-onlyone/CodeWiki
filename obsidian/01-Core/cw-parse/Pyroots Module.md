---
tags: [core, cw-parse, module]
---

# Pyroots Module

> Where Python's import names start: the folders an absolute import is taken from, read from the build files (`pyproject.toml`, `setup.cfg`, `setup.py`) and never run.

## What it does

Finds where Python's import names start. In Python, `import shop.cart` means a folder `shop` inside some root folder. If the scanner guesses the wrong root, imports link to the wrong file. This module reads the build files (`pyproject.toml`, `setup.cfg`, `setup.py`) as data, never runs them, and lists the roots they declare. Supported forms include setuptools `where` and `package-dir`, setup.cfg `package_dir`, Poetry `packages` with `from`, Hatch wheel `packages` and `sources`, PDM `package-dir` and maturin `python-source`. A `src` folder with `.py` files under a project is always a root. A folder below a root counts as part of the dotted path even with no `__init__.py` (PEP 420, the namespace package).

## Source

- `crates/cw-parse/src/pyroots.rs` — module `cw_parse::pyroots`

## How it works

[[PythonRoots]] calls `pyproject_roots`, `setup_cfg_roots` and `setup_py_roots` for each build file. Each parser turns a declared path into a `SourceRoot` through `root`, which joins it to the project folder with `join_norm` and gives `None` if the path climbs out of the repository. A root may carry a prefix: `shop = "lib/shop_impl"` puts the files of `lib/shop_impl` under the package name `shop`. `setup_cfg_roots` reads INI by hand, including continued lines. `setup_py_roots` looks for the literal text `package_dir` followed by `{"": "src"}` and for `where="src"`.

`project_name` is a separate helper. It returns `[project] name`, else `[tool.poetry] name`, from a `pyproject.toml`.

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

## Gotchas

- `setup.py` is read as text. A path computed in code is not followed.
- A pyproject that is not valid TOML gives no roots, with no error.
- The project's own folder is the default and is not listed as a root.
- A file under no declared root falls back to the older rule: the outermost folder with an `__init__.py`, else the project folder, else the repository root minus a leading `src` or `lib`.

## See also

- [[_index]]
- [[cw-parse]]
