---
tags: [integration]
---

# tree_sitter

> External crate `tree_sitter`, imported by 5 source files.

## What it does

Tree-sitter is the parser behind the structural index. It turns source text into a syntax tree, even when the file has errors. The `cw-parse` crate walks that tree to collect facts: symbols, calls, type mentions and imports. One extractor exists for each language: Rust, Python, TypeScript and TSX, and Java. Each language needs its own grammar crate beside this one.

## Used for

Imported by 5 source files. Main users:

- [[Java Module]] — imports `Node`, `Parser`
- [[Lang Module]] — imports `Node`
- [[Python Module]] — imports `Node`, `Parser`
- [[Rust Module]] — imports `Node`, `Parser`
- [[Typescript Module]] — imports `Node`, `Parser`

## Configuration

- `crates/cw-parse/Cargo.toml` — declares the dependency: version `0.27.2`, locked `0.27.2`

## Wire-up

Each extractor builds a `Parser`, calls `set_language` with its grammar, and parses the source. If no tree comes back, it returns an error. It records `root.has_error()` as `has_errors` on the result, so later steps know the facts may be incomplete.

The extractor then walks `Node` values from the root. The [[Lang Module]] holds the shared part for Python, TypeScript and Java. Its `Sink` collects facts, and helpers such as `named` and `field_all` list children and fields of a node. The TypeScript extractor picks the TSX or plain grammar by file type. The [[Rust Module]] has its own walker.

After parsing, `link_imports` decides if each Python or Java import is repository code, an outside package, or the language's own library.

## Auth mode

No sign-in. Parsing is local and needs no network.

## Gotchas

- Tree-sitter does not fail on bad syntax. It returns a tree with error nodes, so check `has_errors` before trusting the facts of a file.
- Node text is read from the source bytes. `utf8_text` can fail, and the helpers then give an empty string.
- Children are collected into a `Vec` before the walk, because the extractor changes its own state while visiting the tree.
- Qualified names use `::` in every language, so one resolver serves all of them.
- Signatures are cut at 300 characters.

## See also

- [[_index]]
