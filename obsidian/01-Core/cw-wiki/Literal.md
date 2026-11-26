---
tags: [core, cw-wiki]
---

# Literal

> One string literal.

## What it does

`Literal` records one string literal found in Rust source: where it opens, where it ends, and the text between the quotes. Scanners use it to look at strings without being fooled by comments or code that merely looks like text. Offsets index into the masked code of a `Scanned` value, in which comments and string contents are blanked.

## Source

- `crates/cw-wiki/src/rustscan.rs` — definition of `Literal`

## How it works

While the source is masked, each plain, byte, C or raw string is cut out. A `Literal` is stored with `open` (the opening quote), `close` (one past the closing quote and any raw-string hashes) and `value` (the text as written, escapes not decoded). The string contents in the code copy are replaced by spaces, so searching the code never matches inside a string. Callers use `Scanned::literal_at` to find the literal that opens at an offset. The contracts scan and the security scan use it to read the first argument of a call.

## Items

Declared as `pub(crate) struct Literal`.

## Used by

- [[Rustscan Module]] — from `mask`, `raw_string`, `string`
- [[Scanned]] — from `Scanned`, `literal_at`
- [[Contracts Module]] — from `first_literal`
- [[Security Module]] — from `leading_literal`

## Gotchas

- `value` keeps escapes as written. A `\n` stays two characters.
- Offsets are byte offsets in the source. They stay valid in the masked code because only ASCII bytes are overwritten.
- An unterminated string ends at the end of the file.

## See also

- [[_index]]
- [[Rustscan Module]]
