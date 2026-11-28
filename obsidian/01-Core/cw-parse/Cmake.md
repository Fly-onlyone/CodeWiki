---
tags: [core, cw-parse]
---

# Cmake

> `Cmake` struct defined in `cw_parse::cmake` with 3 methods.

## What it does

`Cmake` is the result of reading every `CMakeLists.txt` in a repository. It holds the targets (libraries and executables with their sources, include directories and links), the `include_directories` calls per folder, the packages the project asks for, the CTest tests, and the list of files read. Other code asks it two questions: which directories may this source file include from, and which target does this file belong to.

## Source

- `crates/cw-parse/src/cmake.rs` — definition of `Cmake`

## How it works

`read_cmake` in [[Cmake Module]] builds it. The three methods only read the finished value.

`has_targets` is true when any target exists.

`include_dirs` collects directories for one file. It takes the folder-wide `include_directories` calls whose folder contains the file. Then it takes every target that lists the file, or that sits in the file's folder or above it. It adds each target's include directories and follows its links to other targets, with a cap of 200 steps, so an executable sees the include directories of the libraries it links. The list is sorted and deduplicated.

`target_of` picks a target in three tries: one that lists the file; one that lists a file of the same name without its extension (a header next to its source); otherwise the target of the deepest folder that contains the file, with a library winning over an executable at the same depth.

## Items

Declared as `pub struct Cmake`.

Members:

- `pub fn has_targets(&self) -> bool` — true when the repository has a `CMakeLists.txt` that declares a target.
- `pub fn include_dirs(&self, file: &str) -> Vec<String>` — the directories a source file may `#include` from, besides its own: those named for the targets that list it or sit in its folder or above, for the folders it lies in, and for the targets they link.
- `pub fn target_of(&self, file: &str) -> Option<&Target>` — the target that lists `file`, else the one of the nearest folder above it, a library before an executable.

## Depends on

- [[Cmake Module]] — uses `Package`, `Target`, `Test`

## Used by

- [[Cmake Module]] — from `Reader`, `read_cmake`
- [[CPlan]] — from `CPlan`, `cmake`
- [[Cfamily Module]] — from `declared_libraries`
- [[Testmap Module]] — from `Env`
- [[Units Module]] — from `discover`

## Gotchas

- A target with an empty folder (the repository root) counts as containing every file, so root targets add their include directories to all files.
- `target_of` can return a root target for a file that no `CMakeLists.txt` mentions, as a last resort.
- Targets from every `if` branch are present, since conditions are never evaluated, so the answers can be wider than a real build.

## See also

- [[_index]]
- [[Cmake Module]]
