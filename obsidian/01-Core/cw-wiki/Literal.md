---
tags: [core, cw-wiki]
---

# Literal

> One string literal.

## Source

- `crates/cw-wiki/src/rustscan.rs` — definition of `Literal`

## How it works

Declared as `pub(crate) struct Literal`.

Offsets index into `Scanned::code`.

## Used by

- [[Rustscan Module]] — from `mask`, `raw_string`, `string`
- [[Scanned]] — from `Scanned`, `literal_at`
- [[Contracts Module]] — from `first_literal`
- [[Security Module]] — from `leading_literal`

## See also

- [[_index]]
- [[Rustscan Module]]
