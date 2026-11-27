---
tags: [core, cw-parse]
---

# Cmake

> `Cmake` struct defined in `cw_parse::cmake` with 3 methods.

## Source

- `crates/cw-parse/src/cmake.rs` — definition of `Cmake`

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

## See also

- [[_index]]
- [[Cmake Module]]
