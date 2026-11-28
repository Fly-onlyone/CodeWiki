---
tags: [core, cw-parse, module]
---

# Cdeps Module

> The files a C or C++ project names its libraries in, other than `CMakeLists.txt` (see `cmake.rs`): `vcpkg.json`, `conanfile.txt`, `conanfile.py` and `conan.lock`.

## What it does

Reads the files where C and C++ projects list the libraries they use, apart from `CMakeLists.txt`: `vcpkg.json`, `conanfile.txt`, `conanfile.py` and `conan.lock`. Each parser gives back plain `Library` values (a name and a version, empty when none is given). It also holds `names_library`, which decides whether an `#include` path probably belongs to a declared library, so that `#include <fmt/core.h>` can be linked to the package `fmt`.

## Source

- `crates/cw-parse/src/cdeps.rs` — module `cw_parse::cdeps`

## How it works

Four parsers, one per format, share the helper `reference`. It turns `fmt/10.1.1@user/channel#abc` into name `fmt` and version `10.1.1`, dropping the user, channel and revision.

- `parse_vcpkg` uses serde on the JSON. A dependency is a bare name or an object with `name` and `version>=`. It also returns the overrides and the baseline.
- `parse_conanfile_txt` reads only the `[requires]` section. Other sections such as `[tool_requires]` are ignored.
- `parse_conanfile_py` scans lines for `requires = ...` (a string, tuple or list over several lines) and for `self.requires(...)` calls. `string_literals` pulls out the quoted strings.
- `parse_conan_lock` reads the JSON `requires` and `build_requires`.

`names_library` is the matcher. It lowercases and drops punctuation, then accepts an exact match, a match after trimming trailing digits (`Eigen` for `Eigen3`), or a match with the first word of a family name (`boost` for `boost-asio`, `nlohmann` for `nlohmann_json`). Callers are [[Cfamily Module]] and [[CPlan]] (to name the library) and [[Manifests Module]].

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

## Gotchas

- Bad input gives an empty result, never an error. A `vcpkg.json` or `conan.lock` that is not valid JSON returns nothing.
- `parse_conanfile_py` reads plain string literals only. A requirement built from a variable or an f-string is missed. A version range like `zlib/[>=1.2 <2]` gives no library, since the version part is cut at the bracket and ends up empty.
- In `conanfile.txt`, a `#` starts a comment, so `boost/1.83.0#abcdef` is read up to the `#` and the revision is dropped, which is fine.
- `names_library` needs at least two characters in the include name, and refuses weak matches on purpose: an include is linked to a library only when the link is evident.

## See also

- [[_index]]
- [[cw-parse]]
