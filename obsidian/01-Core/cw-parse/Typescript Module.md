---
tags: [core, cw-parse, module]
---

# Typescript Module

> TypeScript and TSX symbol and fact extraction.

## What it does

Written against the node kinds and field names published in the grammar's own `node-types.json` (tree-sitter-typescript 0.23, both dialects). No query files are used.

## Source

- `crates/cw-parse/src/typescript.rs` — module `cw_parse::typescript`

## How it works

1. `scan_repo` in [[Scan Module]] calls `parse_typescript`. `parse_typescript` calls `Error::external`, then `Error::invalid` (both in [[Error]]), then `Sink::new` in [[Sink]], then `named` in [[Lang Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `parse_typescript` — parse one TypeScript or TSX source file. `module` is the qualified module the file defines, for example `src::util::text`. `resolve` maps a relative import specifier written in this file to the module it names, or `None` when no such file is indexed.
- `enum Target` — what an import specifier names.
- `fn pattern_names(node: Node, src: &[u8], out: &mut HashSet<String>)` — names bound by a declaration or parameter pattern.
- `fn bindings(node: Node, src: &[u8], out: &mut HashSet<String>)` — names bound anywhere in a function scope, not descending into nested functions and classes.

And 4 more private items with no comment.

## Depends on

- [[Sink]] — uses `Sink`, `between`, `import_external`, `inherits`
- [[Lang Module]] — uses `block_doc`, `collapse`, `dotted`, `field_all`
- [[Error]] — uses `external`, `invalid`
- [[Error Module]] — uses `Result`
- [[Visibility]] — uses `Visibility`
- [[ParsedFile]] — uses `ParsedFile`

## Used by

- [[Scan Module]] — from `scan_repo`

## See also

- [[_index]]
- [[cw-parse]]
