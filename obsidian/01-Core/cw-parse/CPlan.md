---
tags: [core, cw-parse]
---

# CPlan

> What the scan knows about the C and C++ files of one repository.

## What it does

`CPlan` holds what the scan decides about all C and C++ files of a repository before it parses them. A parser sees one file. This plan sees every path, so it can answer questions no single file can: is this `.h` C or C++, which module does a file belong to, which files are vendored, and what does `#include "x/y.h"` point at (a file in the repository, a declared library, or the platform).

## Source

- `crates/cw-parse/src/cfamily.rs` — definition of `CPlan`

## How it works

`CPlan::build` runs once per scan and returns `None` when there is no C or C++ file.

1. It collects headers and sources. Files under vendored folders such as `third_party` or `vendor` are marked vendored and are not parsed, but only when the repository has code outside them.
2. It decides the language of each file. A `.h` is C++ if `looks_cpp` finds C++-only words outside `#ifdef __cplusplus` blocks. If not, the source of the same name decides, then the folder's sources (C++ only means C++), else C.
3. It names modules with `module_names`, pairing a header with its source of the same stem, even across folders when each is unique.
4. It reads CMake files and package files (`vcpkg.json`, `conanfile.txt`, `conanfile.py`, `conan.lock`) to learn which libraries the repository declares.

Later, `include_dirs` gives the directories an include may be relative to (CMake include directories plus `include`, `src`, `lib` at each level up). `include` then tries a quoted include next to the file, then those directories, then a name only one file can mean. A hit in a vendored tree becomes a library named by its folder. No hit falls to `library`, which says platform headers are unknown and matches the first path part against declared libraries.

## Items

Declared as `pub struct CPlan`.

Members:

- `pub fn build(paths: &[String], read: &dyn Fn(&str) -> Option<String>) -> Option<Self>` — `None` when the repository has no C or C++ file.
- `pub fn include_dirs(&self, file: &str) -> HashSet<String>` — directories an include of `file` may be written relative to, besides its own folder.
- `pub fn cmake(&self) -> &Cmake` — public method, returns `&Cmake`.
- `pub fn include(&self, from: &str, dirs: &HashSet<String>, spec: &str, quoted: bool) -> Include` — what `#include` `spec` means in `from`, given the directories `Self::include_dirs` named for it.
- `fn library(&self, spec: &str) -> Include` — A library the repository declares and `spec` is evidently from.

And 1 more private item with no comment.

## Depends on

- [[Cfamily Module]] — uses `PLATFORM_DIRS`, `PLATFORM_HEADERS`, `declared_libraries`, `dir_of`
- [[Language]] — uses `Language`, `from_path`
- [[C Module]] — uses `Include`
- [[Cdeps Module]] — uses `names_library`
- [[Cmake Module]] — uses `read_cmake`
- [[Cmake]] — uses `Cmake`

## Used by

- [[Cfamily Module]] — from `home_of`, `includes_of`, `looks_cpp`, `merge`
- [[Scan Module]] — from `scan_repo`

## Gotchas

- A repository made only of vendored code is parsed normally. The vendored rule applies only if there is own code.
- An include that matches nothing, or a library the repository never declared, is `Include::Unknown`. It is not guessed.
- Platform headers (`stdio.h`, `sys/types.h`, `windows.h`) and extensionless names (`vector`) are always unknown, never libraries.
- Two sources of one name with no header, in different folders, are separate modules named in full, with `_2` style suffixes if still equal.
- The module count is the same whether or not a file is vendored only through `c_module_map`, which ignores vendored files the same way.

## See also

- [[_index]]
- [[Cfamily Module]]
