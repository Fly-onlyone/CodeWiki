---
tags: [core, cw-wiki, module]
---

# Rustscan Module

> A light lexical scan of Rust source, shared by the test map and the security rules.

## What it does

A small scanner for Rust source, not a parser. The main parser skips unit-test modules on purpose. The test map and the security rules still need to know three things: which bytes are real code, which are string literals, and which belong to test-only items. This module answers them. It produces a [[Scanned]] value in which comments and the insides of literals are replaced by spaces, so a pattern search can never match inside a comment or a string.

## Source

- `crates/cw-wiki/src/rustscan.rs` — module `cw_wiki::rustscan`

## How it works

`mask` walks the source byte by byte. It blanks line comments and nested block comments. It reads normal strings (`string`), raw strings with any number of hashes (`raw_string`), byte and C strings, and character literals (`char_or_lifetime`, which also steps over lifetimes). Blanking keeps newlines and byte offsets, so a position in the masked code is a position in the source. Each string is saved as a [[Literal]] with its open and close offsets.

`find_tests` then scans the masked code for attributes. `classify` sorts them: a test attribute (`test`, `tokio::test`, `rstest`, `test_case`), a `cfg(test)` attribute, or other. For a hit it skips further attributes, walks the item header to its body or semicolon using `matching` for brackets, and records the byte range as a test region. For test attributes it also finds the function name with `fn_name` and saves a [[TestFn]]. An inner attribute `#![cfg(test)]` marks the rest of the file.

## Items

- [[Literal]] — one string literal.
- [[TestFn]] — A `#[test]`-like function found in a file.
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
- [[TestFn]] — uses `TestFn`

## Used by

- [[Refactor Module]] — from `break_cycles`, `extract_types`, `move_functions`, `split_files`
- [[Scanned]] — from `find`, `matching`, `new`, `statement_end`
- [[Contracts Module]] — from `scan`, `scan_source`
- [[Security Module]] — from `scan`, `scan_source`
- [[Coverage Module]] — from `report`
- [[Testmap Module]] — from `is_test_file`

## Gotchas

- `cfg(not(test))` is not counted as a test region.
- After a hit the scan steps past the attribute only, not the item, so tests nested in a test module are found too. Duplicate name and line pairs are skipped.
- Literal values keep escapes as written, and the masked code only blanks ASCII bytes, so non-ASCII text stays valid.
- `is_test_path` here matches `tests`, `benches` and `examples` folders. The planner has its own version that also matches `test` and works on module paths.
- An unclosed bracket makes `matching` return the end of the file.

## See also

- [[_index]]
- [[cw-wiki]]
