---
tags: [core, cw-parse, module]
---

# C Module

> C and C++ symbol and fact extraction, one walker for both languages.

## What it does

Turns one C or C++ file into symbols and facts (calls, type references, uses). A single walker serves both languages. The C++ grammar is the C grammar plus extra node kinds, so a C file simply never meets the C++ ones. `CInput::cpp` picks the grammar.

The parser works on one file only. Anything that needs other files is left to the scan: pairing a header with its source, resolving includes, and merging a declaration with its definition. That part lives in [[Cfamily Module]] and [[CPlan]].

A module is a header and source pair of one name, so `net/socket.h` and `net/socket.c` are the module `net::socket`. Namespaces are not modules; a symbol records its namespace in a `namespace:` attribute. Symbol names are `module::Name`, `module::Class::member` and `module::Enum::NAME`.

Some things are deliberately not symbols: forward declarations, template specializations (their bodies are still walked for calls), include guards and empty `#define`s. Every branch of `#if` is parsed and conditions are never evaluated.

## Source

- `crates/cw-parse/src/c.rs` — module `cw_parse::c`

## How it works

`scan_repo` calls `parse_c` once per file with a `CInput` that names the module and the include resolution. `parse_c` loads the C or C++ grammar, then runs `blank_statement_macros` on the text. That step blanks lone macro lines such as `FT_BEGIN_HEADER`, and the name of wrapper macros such as `FT_EXPORT( T )`, keeping every byte position the same. Without it the grammar would read the next declaration as part of the macro.

The parse tree goes to an `Extractor` that writes into a `Sink`. It adds one module symbol (with the file's leading doc from `file_doc`), then walks the top-level items. Each item becomes a function, type, typedef, enum, macro, constant or test symbol. Calls, type references and includes are recorded as facts on the way.

Tests are found two ways: macros the grammar reads as a function (`TEST(Suite, Name)`), and block macros such as `TEST_CASE`. They get `test:` attributes.

If the tree has errors, the sink is told, and what could be read still stands.

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

## Gotchas

- Visibility is rough. Inside a class it follows `public`, `protected` (shown as `Crate`) and `private`. Outside a class, `static` or an anonymous namespace means `Private`, everything else `Public`. The scan later lowers symbols that only a source file defines.
- An out-of-line definition `Foo::bar` carries a temporary `outline:` attribute. The scan resolves it to the real class and removes it, so a parser result alone has the wrong parent.
- The `declaration` attribute marks a symbol that is only declared (prototype, `extern`). The scan prefers the part that defines it, then strips the attribute.
- Macro blanking only touches a line standing between declarations. A macro in another position is left alone and may confuse the grammar.
- `same_type_name` treats `Foo`, `_Foo`, `foo_t`, `FOO` and `foo` as one name, so `typedef struct _foo {} foo_t;` becomes one symbol.

## See also

- [[_index]]
- [[cw-parse]]
