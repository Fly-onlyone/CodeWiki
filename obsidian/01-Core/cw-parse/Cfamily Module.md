---
tags: [core, cw-parse, module]
---

# Cfamily Module

> What the scan decides about C and C++ before and after it parses a file: which language a `.h` is, which header and source form one module, what an `#include` names, and how the parts of a declaration become one symbol.

## What it does

The walker itself is `c.rs`; this is the part that needs the other files.

The rules:

- **Language.** `.c` is C. `.cc`, `.cpp`, `.cxx`, `.c++`, `.hpp`, `.hh`, `.hxx`, `.ipp`, `.tpp` and `.inl` are C++. A `.h` is C++ when it uses what only C++ has (`class`, `namespace`, `template`, an `#include` of a header with no extension...), outside the `#ifdef __cplusplus` blocks C headers keep for C++ callers; otherwise when the source of the same name is C++, or its folder holds C++ sources and no C; otherwise C.
- **Module.** A header and the source of its name are one module (`net/socket.h` and `net/socket.c`), named from the header's path without its extension and without a leading `src`, `lib` or `include`. A header and a source of one name in different folders (`include/x/a.h` and `src/a.cpp`) are one module when each is the only one of its kind with that name. Every other file is a module of its own.
- **Declaration and definition** of one name in a module are one symbol, whose file is the header and which lists the sources that define it (`impl:<file>`). `void Foo::bar() {}` joins the class `Foo`, wherever the class is declared. A function a header declares and another module defines is merged when the defining module includes that header.
- **Vendored trees** (`third_party`, `vendor`, `external`, `extern`, `deps`, `_deps`, `3rdparty`) are not parsed when the repository has C or C++ outside them: they would dwarf the project in every ranking and in the time of a scan. An include that reaches into one names the library by the folder it sits in.

## Source

- `crates/cw-parse/src/cfamily.rs` — module `cw_parse::cfamily`

## How it works

1. `is_c_api` in [[Health Module]] and `is_c_entry` in [[Units Module]], and 1 more, call `is_header`. `is_header` calls `extension`.
2. `add_c_pair_dependents` in [[Testmap Module]] and `discover` in [[Units Module]] call `c_module_map`. `c_module_map` calls `is_header`, then `is_source`, then `is_vendored`, then `module_names`.
3. `CPlan::build` in [[CPlan]] calls `is_source`. `is_source` calls `extension`.
4. `CPlan::build` and `CPlan::find` in [[CPlan]], and 1 more, call `dir_of`.
5. `CPlan::build` in [[CPlan]] calls `stem_of`.

7 more entry points: `is_vendored`, `vendored_library`, `looks_cpp`, `module_names`, `declared_libraries`, `normalize`, and 1 more.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

## See also

- [[_index]]
- [[cw-parse]]
