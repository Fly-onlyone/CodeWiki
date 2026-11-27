---
tags: [core, cw-parse]
---

# PythonRoots

> The Python projects of a repository and their import roots.

## Source

- `crates/cw-parse/src/pyroots.rs` — definition of `PythonRoots`

## How it works

1. `scan_repo` in [[Scan Module]] and `gather` in [[Testmap Module]], and 1 more, call `read`. `read` calls `parent_dir`, then `join` (both in [[Jsbuild Module]]), then `pyproject_roots`, then `setup_cfg_roots`, then `setup_py_roots` (all in [[Pyroots Module]]).
2. `discover` in [[Units Module]] calls `in_package`. `in_package` calls `parent_dir`, then `join` (both in [[Jsbuild Module]]).

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct PythonRoots`.

Members:

- `pub fn read(paths: &[String], read: &dyn Fn(&str) -> Option<String>) -> Self` — read the build files among `paths` through `read`.
- `pub fn project_of(&self, path: &str) -> Option<&str>` — the folder of the nearest build file above `path`, `""` for the repository root.
- `pub fn in_package(files: &HashSet<&str>, path: &str) -> bool` — true when a folder between `path` and the repository root has an `__init__.py`: the file is part of a regular package.
- `pub fn module(&self, files: &HashSet<&str>, path: &str) -> (String, bool)` — module a Python file defines, and whether the file is a package (`__init__.py`).

## Depends on

- [[Pyroots Module]] — uses `BUILD_FILES`, `SourceRoot`, `pyproject_roots`, `root`
- [[Jsbuild Module]] — uses `ancestors`, `join`, `parent_dir`
- [[Lang Module]] — uses `sanitize`

## Used by

- [[Testmap Module]] — from `Env`, `gather`
- [[Lang Module]] — from `python_module`
- [[Scan Module]] — from `scan_repo`
- [[Units Module]] — from `discover`
- [[PythonTests]] — from `project_dir`

## See also

- [[_index]]
- [[Pyroots Module]]
