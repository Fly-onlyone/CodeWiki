---
tags: [core, cw-wiki]
---

# Scanned

> `Scanned` struct defined in `cw_wiki::rustscan` with 7 methods.

## Source

- `crates/cw-wiki/src/rustscan.rs` — definition of `Scanned`

## How it works

Declared as `pub(crate) struct Scanned`.

Public API:

- `pub fn matching(&self, open: usize) -> usize` — offset one past the bracket closing the one at `open`.
- `pub fn literal_at(&self, open: usize) -> Option<&Literal>` — the literal whose opening quote is at `open`.
- `pub fn line_of(&self, offset: usize) -> u32` — 1-based line of a byte offset.
- `pub fn statement_end(&self, from: usize) -> usize` — end of the statement containing `from`: the next `;` outside nested brackets, or the end of the enclosing block.
- `pub fn in_test(&self, offset: usize) -> bool` — true when `offset` lies in an item compiled only for tests.
- `pub fn find<'a>(&'a self, word: &'a str) -> impl Iterator<Item = usize> + 'a` — offsets where `word` occurs in code as a whole identifier or path.

## Depends on

- [[Rustscan Module]] — uses `find_tests`, `is_ident`, `mask`, `matching`
- [[Literal]] — uses `Literal`
- [[TestFn]] — uses `TestFn`

## Used by

- [[Security Module]] — from `bound_to_format`, `call_args`, `command_injection`, `env_set_var`
- [[Contracts Module]] — from `attribute_routes`, `call_open`, `first_literal`, `fn_body`
- [[Testmap Module]] — from `gather`

## See also

- [[_index]]
- [[Rustscan Module]]
