---
tags: [core, cw-wiki]
---

# TestFn

> A `#[test]`-like function found in a file.

## What it does

A record of one test function found in a Rust file: its name and the line where it is declared. The test map uses these records to link tests to the code they cover.

## Source

- `crates/cw-wiki/src/rustscan.rs` — definition of `TestFn`

## How it works

`find_tests` in [[Rustscan Module]] creates one for each function marked with a test attribute such as `test`, `tokio::test`, `rstest` or `test_case`. [[Scanned]] keeps them in a list called `tests`, in source order.

## Items

Declared as `pub(crate) struct TestFn`.

## Used by

- [[Rustscan Module]] — from `find_tests`
- [[Scanned]] — from `Scanned`

## Gotchas

- The line is that of the `fn` keyword, not of the attribute.
- A function with the same name on the same line is stored once.
- Only functions with a test attribute are listed. A helper inside a test module is not a `TestFn`, though it lies in a test region.

## See also

- [[_index]]
- [[Rustscan Module]]
