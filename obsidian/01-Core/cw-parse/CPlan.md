---
tags: [core, cw-parse]
---

# CPlan

> What the scan knows about the C and C++ files of one repository.

## Source

- `crates/cw-parse/src/cfamily.rs` — definition of `CPlan`

## How it works

1. `scan_repo` in [[Scan Module]] calls `build`. `build` calls `is_header`, then `is_source`, then `is_vendored` (all in [[Cfamily Module]]), then `Language::from_path` in [[Language]], then `dir_of`, then `stem_of` (both in [[Cfamily Module]]), and 4 more.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

## See also

- [[_index]]
- [[Cfamily Module]]
