---
tags: [core, cw-parse, module]
---

# Typescript Module

> TypeScript and TSX symbol and fact extraction.

## Source

- `crates/cw-parse/src/typescript.rs` — module `cw_parse::typescript`

## How it works

Main items:

- `parse_typescript` — parse one TypeScript or TSX source file.
- `Extractor` — struct
- `Ctx` — struct
- `text` — function
- `pattern_names` — names bound by a declaration or parameter pattern.
- `is_function_value` — function
- `bindings` — names bound anywhere in a function scope, not descending into nested functions and classes.
- `Target` — what an import specifier names.

## Depends on

- [[Lang Module]] — uses `block_doc`, `collapse`, `dotted`, `field_all`
- [[Sink]] — uses `Sink`, `between`, `inherits`, `module_symbol`
- [[Error]] — uses `external`, `invalid`
- [[Error Module]] — uses `Result`
- [[Model Module]] — uses `Visibility`
- [[Graph]] — uses `id`
- [[ParsedFile]] — uses `ParsedFile`

## Used by

- [[Scan Module]] — from `scan_repo`

## See also

- [[_index]]
- [[cw-parse]]
