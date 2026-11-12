---
tags: [core, cw-parse, module]
---

# Python Module

> Python symbol and fact extraction.

## Source

- `crates/cw-parse/src/python.rs` — module `cw_parse::python`

## How it works

Main items:

- `parse_python` — parse one Python source file.
- `MAIN_GUARD_ATTR` — attribute recorded on the module symbol of a file that runs as a script: it has a top-level `if __name__ == "__main__":` block.
- `Extractor` — struct
- `text` — function
- `Ctx` — struct
- `pattern_names` — names bound by an assignment target, loop variable or `as` clause.
- `visibility` — function
- `param_names` — function

## Depends on

- [[Lang Module]] — uses `collapse`, `dedent`, `dotted`, `field_all`
- [[Sink]] — uses `Sink`, `between`, `inherits`, `module_symbol`
- [[Error]] — uses `external`, `invalid`
- [[Error Module]] — uses `Result`
- [[Model Module]] — uses `Visibility`
- [[ParsedFile]] — uses `ParsedFile`

## Used by

- [[Scan Module]] — from `scan_repo`
- [[Units Module]] — from `discover`

## See also

- [[_index]]
- [[cw-parse]]
