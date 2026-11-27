---
tags: [core, cw-parse, module]
---

# Cmake Module

> A reader of `CMakeLists.txt`, for what a repository's units need: the targets and the files each lists, the include directories, the links between targets, the libraries the project asks for and the tests it registers with CTest.

## What it does

CMake is a language, and this reads the part of it that projects write as data. It follows `set` and `list(APPEND)` of variables, `file(GLOB)` with a literal pattern, `add_subdirectory`, and the branches of every `if` (none is evaluated, as the preprocessor's are not). It does not run functions or macros, does not read `include()`d files, and does not evaluate generator expressions other than `$<BUILD_INTERFACE:...>`. An argument it cannot resolve is left out: a source it cannot name is a source that falls to its folder's unit.

## Source

- `crates/cw-parse/src/cmake.rs` — module `cw_parse::cmake`

## How it works

1. `gather` in [[Testmap Module]] and `discover` in [[Units Module]], and 1 more, call `read_cmake`. `read_cmake` calls `commands`, then `locate`. `commands` calls `skip_comment`, then `arguments`. `locate` calls `normalize`, then `join`.
2. `cfamily` in [[Manifests Module]] calls `declared_packages`. `declared_packages` calls `commands`, then `found`, then `fetched`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub enum TargetKind` — what kind of thing a target builds.
- `pub struct Target` — an `add_library` or `add_executable`.
- `pub enum PackageSource` — where a library the project asks for is declared.
- `pub struct Package` — public struct; used by `Cmake`, `declared_packages`, `fetched`, `found`.
- `pub struct Test` — A test registered with CTest.
- [[Cmake]] — struct
- `fn commands(text: &str) -> Vec<Command>` — split a CMake file into its commands, comments left out. A quoted argument loses its quotes; `[[...]]` arguments are kept as written.
- `fn skip_comment(b: &[u8], mut i: usize) -> usize` — private function, returns `usize`; called by `arguments`, `commands`.
- `fn arguments(text: &str, mut i: usize) -> (Vec<String>, usize)` — the arguments after an opening parenthesis, and where the command ends.
- `const ROOT_MARK: char` — marks a path in a variable as named from the repository root, which is how CMake's own folder variables read.
- `fn locate(dir: &str, item: &str) -> Option<String>` — A path as written in a `CMakeLists.txt` in `dir`, repo-relative.
- `fn join(dir: &str, path: &str) -> String` — private function, returns `String`; called by `Reader::command`, `Reader::directory`, `locate`.
- `fn normalize(path: &str) -> Option<String>` — `a/./b/../c` as `a/c`; `None` when the path leaves the repository or is absolute.
- `pub fn read_cmake(paths: &[String], read: &dyn Fn(&str) -> Option<String>) -> Cmake` — read every `CMakeLists.txt` of the repository. `paths` are all its files, sorted; `read` gives the text of one.
- `pub fn declared_packages(text: &str, file: &str) -> Vec<Package>` — the libraries one `CMakeLists.txt` asks for, read from its text alone: no variable is followed, so a name or version written through one is missed. For the build file of a unit, which has no other files at hand.
- `fn found(words: &[String], file: &str) -> Option<Package>` — private function, returns `Option<Package>`; called by `Reader::command`, `declared_packages`.
- `fn fetched(command: &str, words: &[String], file: &str) -> Option<Package>` — private function, returns `Option<Package>`; called by `Reader::fetch`, `declared_packages`.
- `fn wildcard(pattern: &str, name: &str) -> bool` — `*` and `?` in a file name.

And 5 more private items with no comment.

## Depends on

- [[Cmake]] — uses `Cmake`

## Used by

- [[Testmap Module]] — from `gather`, `registration`, `target`
- [[Cmake]] — from `Cmake`, `include_dirs`, `target_of`
- [[Manifests Module]] — from `cfamily`
- [[Units Module]] — from `discover`
- [[CPlan]] — from `build`

## See also

- [[_index]]
- [[cw-parse]]
