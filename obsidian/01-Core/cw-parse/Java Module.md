---
tags: [core, cw-parse, module]
---

# Java Module

> Java symbol and fact extraction.

## Source

- `crates/cw-parse/src/java.rs` — module `cw_parse::java`

## How it works

Main items:

- `parse_java` — parse one Java source file.
- `Extractor` — struct
- `Modifiers` — struct
- `Ctx` — struct
- `is_type_decl` — function

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
