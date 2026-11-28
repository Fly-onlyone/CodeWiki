---
tags: [core, cw-parse, module]
---

# Cfamily Module

> What the scan decides about C and C++ before and after it parses a file: which language a `.h` is, which header and source form one module, what an `#include` names, and how the parts of a declaration become one symbol.

## What it does

The walker in [[C Module]] reads one C or C++ file. This module does the work that needs all files: it decides whether a `.h` is C or C++, groups a header and its source into one module, resolves what each `#include` means, skips vendored code, and merges the pieces of a declaration (prototype, definition, out-of-line method) into one symbol. It is the shared helper layer for the scan, the units, the test map and the health checks.

## Source

- `crates/cw-parse/src/cfamily.rs` — module `cw_parse::cfamily`

## How it works

Two phases around the parse.

Before parsing, [[CPlan]] `build` classifies every file, names modules with `module_names`, and reads CMake and package files. `c_module_map` is a lighter form that returns only the module of each file, for callers that need no includes.

After parsing, `merge` takes every file's symbols and:

1. Gives each out-of-line definition (`Foo::bar`) its class. It looks in the same module first, then by name across the repository.
2. Folds symbols with the same qualified name into one. Headers go first, so the symbol sits where it is declared, and the defining sources are listed as `impl:` attributes (`join_parts`).
3. Merges a function that one module's header declares and another module defines, when the defining module reaches that header through includes (`reach`, up to three levels).
4. Lowers visibility for things only a source file uses: macros become private, other public functions and statics become crate level, except entry points such as `main`.
5. Rewrites calls and references that named a renamed symbol.

Small helpers (`is_header`, `is_source`, `dir_of`, `stem_of`, `is_vendored`) work on path text only.

## Items

- `pub const VENDORED_DIRS: [&str; 9]` — folders that hold other people's code.
- `const CONTAINER_DIRS: [&str; 6]` — folders whose name is no part of a module's name.
- `const PLATFORM_DIRS: &[&str]` — folders of the platform's headers (`sys/types.h`, `netinet/in.h`).
- `pub fn is_header(path: &str) -> bool` — public function, returns `bool`; called by `CPlan::build`, `c_module_map`, `declares_for_others`, `is_c_api` and 2 more.
- `pub fn is_source(path: &str) -> bool` — public function, returns `bool`; called by `CPlan::build`, `c_module_map`, `module_names`.
- `fn extension(path: &str) -> Option<String>` — private function, returns `Option<String>`; called by `is_header`, `is_source`.
- `fn dir_of(path: &str) -> &str` — private function, returns `&str`; called by `CPlan::build`, `CPlan::find`, `CPlan::include_dirs`, `module_names`.
- `fn stem_of(path: &str) -> &str` — private function, returns `&str`; called by `CPlan::build`, `module_names`.
- `fn is_vendored(path: &str) -> bool` — private function, returns `bool`; called by `CPlan::build`, `c_module_map`.
- `fn vendored_library(path: &str) -> Option<String>` — the folder of a vendored file that names its library: the one under the vendor folder (`third_party/fmt/include/fmt/core.h` is `fmt`).
- `pub fn looks_cpp(text: &str) -> bool` — whether a header's text uses what only C++ has. The blocks C headers keep for C++ callers (`#ifdef __cplusplus`) are left out.
- `fn strip_comments(line: &str, in_comment: &mut bool) -> String` — `code` of a line without its comments and string contents.
- [[CPlan]] — what the scan knows about the C and C++ files of one repository.
- `pub fn c_module_map(paths: &[String]) -> HashMap<String, String>` — the module of every C and C++ file of a repository, by path; a file in a vendored tree has none. What the scan and the units both go by.
- `fn module_names(own: &[&&String]) -> HashMap<String, String>` — private function, returns `HashMap<String, String>`; called by `CPlan::build`, `c_module_map`.
- `declared_libraries` — the libraries the repository declares, in the order it does.
- `fn normalize(path: &str) -> Option<String>` — private function, returns `Option<String>`; called by `CPlan::find`.
- `fn module_name(path: &str, strip: bool) -> String` — the module a file's name makes: its path without the extension and, when `strip`, without the folders that only say where sources and headers go (`src`, `lib`, `include`).
- `pub fn declares_for_others(file: &str) -> bool` — true for a file of a header's kind, where a name is declared for others.
- `pub fn merge(files: &mut [(String, ParsedFile)], plan: &CPlan) -> Vec<Symbol>` — apply what only all the files together tell: the class an out-of-line definition belongs to, one symbol for a declaration and its definition, and the modules. Returns the symbols of every C and C++ file; the facts that mention a symbol that was renamed are rewritten in `files`.

And 7 more items.

## Depends on

- [[C Module]] — uses `DECLARATION_ATTR`, `ENTRY_NAMES`, `IMPL_ATTR`, `OUTLINE_ATTR`
- [[Cdeps Module]] — uses `parse_conan_lock`, `parse_conanfile_py`, `parse_conanfile_txt`, `parse_vcpkg`
- [[SymbolKind]] — uses `SymbolKind`, `is_type`
- [[CPlan]] — uses `CPlan`, `find`
- [[Lang Module]] — uses `sanitize`
- [[Symbol]] — uses `Symbol`
- [[Cmake]] — uses `Cmake`
- [[ParsedFile]] — uses `ParsedFile`

## Used by

- [[CPlan]] — from `build`, `find`, `include`, `include_dirs`
- [[Units Module]] — from `discover`, `is_c_entry`
- [[Scan Module]] — from `scan_repo`
- [[Health Module]] — from `is_c_api`
- [[Testmap Module]] — from `add_c_pair_dependents`

## Gotchas

- Vendored folders (`third_party`, `vendor`, `external`, `deps` and the like) are skipped only when the repository has C or C++ outside them.
- `merge` empties the `symbols` of every C file in `files` and returns them all. A caller must use the returned list, not the per-file one.
- Matching a header to a source in different folders works only when each is the only one of its name. Two such pairs stay separate and get `_2` style suffixes.
- Include reach is limited to three hops, so a deep include chain can leave a definition unmerged.
- Conditions of `#if` are never evaluated, in this module or in the walker.

## See also

- [[_index]]
- [[cw-parse]]
