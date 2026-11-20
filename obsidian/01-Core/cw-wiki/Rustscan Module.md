---
tags: [core, cw-wiki, module]
---

# Rustscan Module

> A light lexical scan of Rust source, shared by the test map and the security rules.

## What it does

The structural parser deliberately skips unit-test modules, and neither analysis needs a syntax tree: they need to know which bytes are code, which are string literals, and which belong to test-only items. Comments and literal contents are blanked in `Scanned::code` so a pattern can never match inside either; offsets and line breaks are preserved.

## Source

- `crates/cw-wiki/src/rustscan.rs` — module `cw_wiki::rustscan`

## How it works

1. `scan` and `scan_source` in [[Contracts Module]], and 8 more, call `is_test_path`.
2. `Scanned::new` in [[Scanned]] calls `mask`. `mask` calls `blank`, then `string`, then `char_or_lifetime`, then `is_ident`, then `raw_string`. `string` calls `blank`. `char_or_lifetime` calls `blank`.
3. `Scanned::matching` and `Scanned::statement_end` in [[Scanned]] call `matching`.
4. `Scanned::new` in [[Scanned]] calls `find_tests`. `find_tests` calls `matching`, then `classify`, then `fn_name`. `classify` calls `is_ident`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- [[Literal]] — one string literal.
- `pub(crate) struct TestFn` — A `#[test]`-like function found in a file.
- [[Scanned]] — struct
- `pub(crate) fn is_test_path(path: &str) -> bool` — true for paths under a `tests`, `benches` or `examples` directory.
- `fn is_ident(byte: u8) -> bool` — private function, returns `bool`; called by `Scanned::find`, `classify`, `fn_name`, `mask`.
- `fn blank(code: &mut [u8], from: usize, to: usize)` — private function; called by `char_or_lifetime`, `mask`, `raw_string`, `string`.
- `fn mask(source: &str) -> (String, Vec<Literal>)` — blank comments and literal contents; collect string literals.
- `fn string(source: &str, code: &mut [u8], literals: &mut Vec<Literal>, open: usize) -> usize` — consume a `"…"` literal whose opening quote is at `open`.
- `raw_string` — consume a raw string whose opening quote is at `open`, closed by a quote and `hashes` hash signs.
- `fn char_or_lifetime(source: &str, code: &mut [u8], open: usize) -> usize` — blank a character literal starting at `open`, or step over a lifetime.
- `fn matching(code: &[u8], open: usize) -> usize` — offset one past the bracket that closes the one at `open`.
- `enum Attr` — what an attribute says about the item it is on.
- `fn classify(attr: &str) -> Attr` — private function, returns `Attr`; called by `find_tests`.
- `fn find_tests(code: &str, line_of: impl Fn(usize) -> u32) -> (Vec<TestFn>, Vec<(usize, usize)>)` — find test functions and test-only byte ranges in masked code.
- `fn fn_name(code: &str, from: usize, to: usize) -> Option<(String, usize)>` — name and offset of the function declared in `code[from..to]`.

## Depends on

- [[Literal]] — uses `Literal`

## Used by

- [[Scanned]] — from `Scanned`, `find`, `matching`, `new`
- [[Refactor Module]] — from `break_cycles`, `extract_types`, `move_functions`, `split_files`
- [[Contracts Module]] — from `scan`, `scan_source`
- [[Security Module]] — from `scan`, `scan_source`
- [[Coverage Module]] — from `report`
- [[Testmap Module]] — from `gather`

## See also

- [[_index]]
- [[cw-wiki]]
