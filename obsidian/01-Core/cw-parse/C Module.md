---
tags: [core, cw-parse, module]
---

# C Module

> C and C++ symbol and fact extraction, one walker for both languages.

## What it does

Written against the node kinds and field names published in the grammars' own `node-types.json` (tree-sitter-c 0.24 and tree-sitter-cpp 0.23). No query files are used.

One walker serves both languages because the C++ grammar is the C one plus more node kinds: every C construct has the same shape in both, and the walk simply never meets the C++ kinds in a C file. The grammar is chosen by `CInput::cpp`.

What a module is:

There are no packages. A module is the header/source pair of one name: `src/net/socket.h` and `src/net/socket.c` (or `.cpp`) are the module `net::socket`, the declaration in the header and the definition in the source one symbol (the scan merges them; see `scan.rs`). A header with no source, or a source with no header, is a module of its own. The scan names the module and tells the parser, because pairing needs the other files. Namespaces are not modules: a namespace spans many files, and in most projects there is one. A symbol records the namespace it was declared in as the attribute `namespace:a::b`, and a qualified name such as `a::b::f()` is matched by the resolver against it.

Names:

A symbol is `<module>::<Name>`; a member is `<module>::<Class>::<name>` and an enumerator `<module>::<Enum>::<NAME>`. An out-of-line definition `void Foo::bar() {}` is a method of `Foo` in the module of the file that defines it; the scan fixes the class chain once it knows the classes.

What is kept out:

Forward declarations of types (`struct Foo;`) name something another header owns. A specialization is not a symbol: its body is walked for the calls it makes, on behalf of the primary template. Include guards and empty `#define`s are no symbols; an object-like macro with a value is a constant and a function-like macro is a macro. Code under `#if` and `#ifdef` is parsed in every branch, and a name declared in several is kept once by the scan. Conditions are never evaluated.

## Source

- `crates/cw-parse/src/c.rs` — module `cw_parse::c`

## How it works

1. `scan_repo` in [[Scan Module]] calls `parse_c`. `parse_c` calls `Error::external` in [[Error]], then `blank_statement_macros`, then `Error::invalid` in [[Error]], then `Sink::new` in [[Sink]], then `file_doc`. `blank_statement_macros` calls `macro_name`, then `is_comment`, then `arguments`. `file_doc` calls `clean_block`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub enum Include` — what an `#include` names, as the scan decides it from the files of the repository and what its build files declare.
- `pub struct CInput<'a>` — what the scan tells the parser about one file.
- `pub const DECLARATION_ATTR: &str` — attribute of a symbol that is only declared where it stands: a prototype, an `extern` variable, an opaque typedef. The scan prefers the part that defines it.
- `pub const NAMESPACE_ATTR: &str` — attribute prefix: the namespace a symbol was declared in.
- `pub const OUTLINE_ATTR: &str` — attribute prefix, set by the parser and removed by the scan: the class an out-of-line definition names before its name (`Foo` of `Foo::bar`).
- `pub const IMPL_ATTR: &str` — attribute prefix: the file that defines a symbol a header declares.
- `pub const TEST_ATTR: &str` — attribute of a test: `test:<framework>`, with the test's own name in `test-name:<name>` and the group it belongs to in `test-suite:<suite>`.
- `pub const TEST_FRAMEWORK_ATTR: &str` — public constant; used by `CTests::framework`, `Extractor::test_symbol`.
- `pub const TEST_NAME_ATTR: &str` — public constant; used by `CTests::name_of`, `Extractor::test_symbol`.
- `pub const TEST_SUITE_ATTR: &str` — public constant; used by `Extractor::test_symbol`.
- `const FUNCTION_TESTS: [(&str, &str); 7]` — macros of the test frameworks that the grammar reads as a function definition: `TEST(Suite, Name) { ... }`. The framework and whether the first argument is a group (fixture, suite) or the name itself.
- `pub const ENTRY_NAMES: [&str; 4]` — the entry points a program starts at.
- `pub fn parse_c(file: &str, source: &str, input: &CInput) -> Result<ParsedFile>` — parse one C or C++ source file.
- `fn blank_statement_macros(source: &str) -> Option<String>` — the source with the macros that the grammar cannot read set aside, byte for byte, or `None` when there is none.
  - A macro that stands alone on its line (`FT_BEGIN_HEADER`, `BSSL_NAMESPACE_BEGIN`) has no semicolon, and the grammar then reads the declaration after it as part of it: the line is blanked.
  - A macro that wraps a return type (`FT_EXPORT( FT_Error ) name( ... )`) is read as a statement followed by junk: its name and parentheses are blanked, and the type stays.
  Only a line that stands between declarations is touched: what comes before it ends a statement, a block or a preprocessor line, and what comes after it does not close or continue a list.
- `fn file_doc(source: &str) -> String` — A leading `//!` block or a `/*! */` or `/** @file */` comment: the documentation of the file. Other comments at the top are mostly licenses, and a `/** */` comment is the documentation of the declaration below it.
- `fn clean_block(inner: &str) -> String` — private function, returns `String`; called by `comment_text`, `file_doc`.
- `struct Declared<'t>` — what a declarator names.
- `fn is_constant_name(name: &str) -> bool` — A name that by convention is a constant: `MAX_SIZE`, `kDefault`.
- `pub(crate) fn same_type_name(a: &str, b: &str) -> bool` — `Foo`, `_Foo`, `foo_t`, `FOO` and `foo` are one name for the purpose of `typedef struct _foo { ... } foo_t;`.
- `fn comment_text(raw: &str) -> String` — A comment's text without its markers.

And 8 more items.

## Depends on

- [[Sink]] — uses `Sink`, `between`, `import_external`, `inherits`
- [[Lang Module]] — uses `block_doc`, `collapse`, `field_all`, `named`
- [[Error]] — uses `external`, `invalid`
- [[Error Module]] — uses `Result`
- [[Symbol]] — uses `Symbol`
- [[Visibility]] — uses `Visibility`
- [[ParsedFile]] — uses `ParsedFile`

## Used by

- [[Testmap Module]] — from `command`, `framework`, `name_of`, `tests`
- [[Cfamily Module]] — from `join_parts`, `merge`
- [[Units Module]] — from `discover`, `is_c_entry`
- [[CPlan]] — from `include`, `library`
- [[Scan Module]] — from `scan_repo`
- [[Plan Module]] — from `files_of`
- [[Render Module]] — from `implementation_files`
- [[Resolver]] — from `see_c_modules`

## See also

- [[_index]]
- [[cw-parse]]
