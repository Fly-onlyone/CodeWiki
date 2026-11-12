---
tags: [core, cw-parse]
---

# UseImport

> One name brought into scope by a `use` declaration.

## Source

- `crates/cw-parse/src/rust.rs` — definition of `UseImport`

## How it works

Declared as `pub struct UseImport`.

## Used by

- [[Lang Module]] — from `external`, `link_java`, `link_python`
- [[Rust Module]] — from `flatten_use`
- [[Sink]] — from `import`
- [[ParsedFile]] — from `ParsedFile`

## See also

- [[_index]]
- [[Rust Module]]
