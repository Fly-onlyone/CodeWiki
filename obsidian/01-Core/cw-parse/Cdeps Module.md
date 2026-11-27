---
tags: [core, cw-parse, module]
---

# Cdeps Module

> The files a C or C++ project names its libraries in, other than `CMakeLists.txt` (see `cmake.rs`): `vcpkg.json`, `conanfile.txt`, `conanfile.py` and `conan.lock`.

## What it does

Formats, from the tools' own documentation:

- vcpkg's manifest (`vcpkg.json`): `dependencies` is a list of names or of objects with `name` and optionally `version>=`; `overrides` is a list of `{ "name", "version" }`; `builtin-baseline` is a commit of the registry.
- Conan 2's `conanfile.txt`: a `[requires]` section with one `name/version` per line, an optional `@user/channel` or `#revision` after it, and `[tool_requires]` for build tools.
- `conanfile.py`: `requires = "name/version"` or a tuple or list of those, and `self.requires("name/version")` in `requirements()`.
- `conan.lock`: JSON with `requires` (and `build_requires`), each `"name/version#revision%timestamp"`.

## Source

- `crates/cw-parse/src/cdeps.rs` — module `cw_parse::cdeps`

## How it works

1. `declared_libraries` in [[Cfamily Module]] and `cfamily` in [[Manifests Module]] call `parse_vcpkg`.
2. `declared_libraries` in [[Cfamily Module]] and `cfamily` in [[Manifests Module]] call `parse_conanfile_txt`. `parse_conanfile_txt` calls `reference`.
3. `declared_libraries` in [[Cfamily Module]] and `cfamily` in [[Manifests Module]] call `parse_conanfile_py`. `parse_conanfile_py` calls `string_literals`, then `reference`.
4. `declared_libraries` in [[Cfamily Module]] and `cfamily` in [[Manifests Module]] call `parse_conan_lock`. `parse_conan_lock` calls `reference`.
5. `same_library` in [[Manifests Module]] and `CPlan::library` in [[CPlan]] call `names_library`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub struct Library` — A library named in a manifest and the version it asks for, if it says.
- `pub struct Vcpkg` — public struct; used by `parse_vcpkg`.
- `pub fn parse_vcpkg(text: &str) -> Vcpkg` — read a `vcpkg.json`. A file that is not JSON gives nothing.
- `fn reference(text: &str) -> Option<Library>` — `fmt/10.1.1@user/channel#abc` as a library.
- `pub fn parse_conanfile_txt(text: &str) -> Vec<Library>` — the `[requires]` of a `conanfile.txt`.
- `pub fn parse_conanfile_py(text: &str) -> Vec<Library>` — the `requires` of a `conanfile.py`: the class attribute and the calls in `requirements()`. Only plain string literals are read.
- `fn string_literals(text: &str) -> Vec<String>` — private function, returns `Vec<String>`; called by `parse_conanfile_py`.
- `pub fn parse_conan_lock(text: &str) -> Vec<Library>` — the libraries a `conan.lock` pins.
- `pub fn names_library(include: &str, library: &str) -> bool` — true when an include's first path segment (or a header's own name) is evidently the library `library`: the names agree once case and punctuation are put aside, or a library is one of a family (`boost-asio` of `boost`, `nlohmann_json` of `nlohmann`), or a digit differs (`Eigen3` of `Eigen`).

And 3 more private items with no comment.

## Used by

- [[Manifests Module]] — from `cfamily`, `same_library`
- [[Cfamily Module]] — from `declared_libraries`
- [[CPlan]] — from `library`

## See also

- [[_index]]
- [[cw-parse]]
