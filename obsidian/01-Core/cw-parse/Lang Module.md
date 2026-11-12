---
tags: [core, cw-parse, module]
---

# Lang Module

> Plumbing shared by the Python, TypeScript and Java extractors: the fact sink, text helpers, the file-to-module mapping and import classification.

## Source

- `crates/cw-parse/src/lang.rs` — module `cw_parse::lang`

## How it works

Main items:

- `python_module` — module a Python file defines, and whether the file is a package (`__init__.py`).
- `resolve_ts_import` — file a relative TypeScript import specifier names, as written in `from`.
- `ts_module` — module a TypeScript file defines: its path without the extension.
- `named` — named children of `node`, collected so the tree can be walked while the extractor is mutated.
- [[Sink]] — collects the facts of one file.
- `MAX_SIGNATURE_CHARS` — const
- `collapse` — declaration text with whitespace collapsed and the body opener removed.
- `block_doc` — text of a `/** ... */` comment with the markers and leading stars removed.

## Depends on

- [[Language]] — uses `Language`, `from_path`
- [[Rust Module]] — uses `ModDecl`
- [[ParsedFile]] — uses `ParsedFile`
- [[UseImport]] — uses `UseImport`

## Used by

- [[Typescript Module]] — from `bindings`, `children`, `class`, `class_body`
- [[Java Module]] — from `children`, `doc`, `enum_constant`, `field`
- [[Python Module]] — from `bindings`, `block`, `class`, `collect_roots`
- [[Sink]] — from `inherits`, `type_ref`
- [[Scan Module]] — from `scan_repo`

## See also

- [[_index]]
- [[cw-parse]]
