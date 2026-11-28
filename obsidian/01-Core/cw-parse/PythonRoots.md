---
tags: [core, cw-parse]
---

# PythonRoots

> The Python projects of a repository and their import roots.

## What it does

Holds the Python projects of a repository and the import roots of each. Given the path of a Python file, it says which dotted module that file defines, for example `shop::cart`, and whether the file is a package (`__init__.py`). The scanner uses this so that `import shop.cart` finds the right file. Roots are described in [[Pyroots Module]].

## Source

- `crates/cw-parse/src/pyroots.rs` — definition of `PythonRoots`

## How it works

`read` first finds every folder that holds a build file (`pyproject.toml`, `setup.py` or `setup.cfg`). For each such project it collects roots: a `src` folder that holds `.py` files, plus whatever the three build files declare. Duplicates and the project folder itself are left out.

`project_of` finds the nearest project folder above a file. `module` then picks the declared root with the longest folder that holds the file, drops the folders above that root, and joins the rest with `::`. A root prefix comes first. With no declared root, the name starts at the outermost folder with an `__init__.py`; failing that, at the project folder; failing that, at the repository root, minus a leading `src` or `lib`. A file `__init__.py` names its folder and the second value is true. `in_package` is a plain check: does a folder between the file and the root hold an `__init__.py` or `.pyi`?

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

## Gotchas

- `module` never returns an empty name. A file with no segments is named `__init__`.
- Segments go through `sanitize`, so odd characters in folder names become safe ones.
- A repository with no build file still works, through the fallback rule.

## See also

- [[_index]]
- [[Pyroots Module]]
