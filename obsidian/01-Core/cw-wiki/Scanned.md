---
tags: [core, cw-wiki]
---

# Scanned

> `Scanned` struct defined in `cw_wiki::rustscan` with 7 methods.

## What it does

The result of scanning one Rust file with [[Rustscan Module]]. It holds a masked copy of the source where comments and string contents are spaces, the string literals, the test functions, and the byte ranges that exist only in test builds. The security rules, the contract finder and the test map use it to search code safely and to ignore test code.

## Source

- `crates/cw-wiki/src/rustscan.rs` — definition of `Scanned`

## How it works

`new` masks the source, builds a table of line starts, and finds tests. Queries then work on byte offsets of the masked code. `find` returns offsets where a word occurs as a whole identifier or path, so `env` does not match `environment`. Words that start or end with a symbol are cut only on the identifier side. `matching` and `statement_end` walk brackets to find where a block or statement ends. `line_of` is a binary search over line starts. `in_test` checks whether an offset lies inside a test region. `literal_at` finds a literal by its opening quote.

## Items

Declared as `pub(crate) struct Scanned`.

Members:

- `pub fn new(source: &str) -> Self` — public method, returns `Self`; called by `gather`, `scan_source`.
- `pub fn line_of(&self, offset: usize) -> u32` — 1-based line of a byte offset.
- `pub fn in_test(&self, offset: usize) -> bool` — true when `offset` lies in an item compiled only for tests.
- `pub fn literal_at(&self, open: usize) -> Option<&Literal>` — the literal whose opening quote is at `open`.
- `pub fn find<'a>(&'a self, word: &'a str) -> impl Iterator<Item = usize> + 'a` — offsets where `word` occurs in code as a whole identifier or path.
- `pub fn matching(&self, open: usize) -> usize` — offset one past the bracket closing the one at `open`.
- `pub fn statement_end(&self, from: usize) -> usize` — end of the statement containing `from`: the next `;` outside nested brackets, or the end of the enclosing block.

## Depends on

- [[Rustscan Module]] — uses `find_tests`, `is_ident`, `mask`, `matching`
- [[Literal]] — uses `Literal`
- [[TestFn]] — uses `TestFn`

## Used by

- [[Security Module]] — from `bound_to_format`, `call_args`, `command_injection`, `env_set_var`
- [[Contracts Module]] — from `attribute_routes`, `call_open`, `first_literal`, `fn_body`
- [[Testmap Module]] — from `gather`

## Gotchas

- All offsets are byte offsets into the masked code, and they equal offsets into the original source.
- `find` never matches inside comments or strings, because they are blanked. Look in `literals` for string contents.
- `statement_end` stops at the first unbalanced closing bracket, so it also ends at the end of the enclosing block.
- Fields `line_starts` and `test_regions` are private. Use `line_of` and `in_test`.

## See also

- [[_index]]
- [[Rustscan Module]]
