---
tags: [core, cw-parse, module]
---

# Cmake Module

> A reader of `CMakeLists.txt`, for what a repository's units need: the targets and the files each lists, the include directories, the links between targets, the libraries the project asks for and the tests it registers with CTest.

## What it does

Reads `CMakeLists.txt` files and returns the build facts the scan needs for C and C++: targets and the source files each one lists, include directories, links between targets, libraries the project asks for, and tests registered with CTest. The result is a [[Cmake]] value. The scan uses it to find which directories an `#include` may be written relative to, and the unit and test-map code uses it to group files by target.

CMake is a full language. This reader handles only the part that projects write as data, and it never runs anything.

## Source

- `crates/cw-parse/src/cmake.rs` — module `cw_parse::cmake`

## How it works

`read_cmake` gets all repository paths and a function that reads a file's text.

1. `commands` splits each file into commands with their arguments. It drops comments (line and `#[[ ]]` block). Quoted and bracket arguments are marked with a hidden prefix so they stay one item when a variable expands.
2. It finds every folder another file adds with `add_subdirectory`. Those are not roots.
3. For each root folder, a `Reader` walks the commands in order and keeps a variable table. It follows `set`, `unset`, `list(APPEND, PREPEND, REMOVE_ITEM)`, `file(GLOB)` and `GLOB_RECURSE` with literal patterns, and `add_subdirectory` (to depth 16, with variables copied into the child). `PARENT_SCOPE` values are handed back to the caller.
4. Command effects fill the result: `add_library` and `add_executable` make targets, `target_sources`, `target_include_directories` and `target_link_libraries` extend them, `find_package`, `FetchContent_Declare` and CPM calls add packages, and `add_test` and the gtest, catch and doctest discovery commands add tests.
5. At the end, alias names are replaced by the real target, and lists are sorted and deduplicated.

`declared_packages` is a lighter path for one file's text alone. `Cmake::include_dirs` and `Cmake::target_of` answer questions for a given source file.

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

## Gotchas

- No `if` is evaluated, so the targets from every branch are all recorded.
- A source that cannot be resolved is silently left out. Variables set by `include()`d files, functions or macros are invisible. Function and macro bodies are skipped.
- Only `$<BUILD_INTERFACE:...>` is understood among generator expressions. Any other `$<...>` makes that argument disappear.
- Only files that exist in the repository path list are accepted as sources. A glob uses the same list, not the disk.
- `declared_packages` follows no variables. A package name written through `${VAR}` is skipped there, though `read_cmake` may resolve it.
- Paths that leave the repository or are absolute are dropped by `normalize`.

## See also

- [[_index]]
- [[cw-parse]]
