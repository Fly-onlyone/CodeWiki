---
tags: [core, cw-parse, module]
---

# Python Module

> Python symbol and fact extraction.

## What it does

Written against the node kinds and field names published in the grammar's own `node-types.json` (tree-sitter-python 0.25). No query files are used.

## Source

- `crates/cw-parse/src/python.rs` — module `cw_parse::python`

## How it works

1. `scan_repo` in [[Scan Module]] calls `parse_python`. `parse_python` calls `Error::external`, then `Error::invalid` (both in [[Error]]), then `Sink::new` in [[Sink]], then `has_main_guard`, then `bindings`. `has_main_guard` calls `named` in [[Lang Module]], then `text`. `bindings` calls `named` in [[Lang Module]], then `pattern_names`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `parse_python` — parse one Python source file. `module` is the qualified module the file defines, for example `pkg::util`; `is_package` is true for `__init__.py`, which changes what a relative import is relative to. Imports are recorded as written (relative ones made absolute); the scanner decides afterwards which of them name repository modules.
- `pub const MAIN_GUARD_ATTR: &str` — attribute recorded on the module symbol of a file that runs as a script: it has a top-level `if __name__ == "__main__":` block.
- `fn has_main_guard(root: Node, src: &[u8]) -> bool` — true when the file has a top-level `if __name__ == "__main__":`.
- `fn text<'a>(node: Node, src: &'a [u8]) -> &'a str` — private function, returns `&'a str`; called by `has_main_guard`, `pattern_names`.
- `fn pattern_names(node: Node, src: &[u8], out: &mut HashSet<String>)` — names bound by an assignment target, loop variable or `as` clause.
- `fn bindings(node: Node, src: &[u8], out: &mut HashSet<String>)` — names bound anywhere in a scope, not descending into nested scopes.

And 4 more private items with no comment.

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
