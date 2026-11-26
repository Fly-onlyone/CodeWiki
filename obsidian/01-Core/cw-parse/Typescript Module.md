---
tags: [core, cw-parse, module]
---

# Typescript Module

> TypeScript and TSX symbol and fact extraction.

## What it does

This module reads one TypeScript, TSX, JavaScript or JSX file and turns it into a [[ParsedFile]]: the module, its functions, classes, interfaces, enums, type aliases, constants and namespaces, plus the calls, type mentions, imports and inheritance in them. It is the front end for the whole JavaScript family in the scanner. It uses tree-sitter and the node names published in the grammar's `node-types.json`.

## Source

- `crates/cw-parse/src/typescript.rs` — module `cw_parse::typescript`

## How it works

`parse_typescript` picks a grammar by file extension. `.tsx`, `.js`, `.jsx`, `.mjs` and `.cjs` use the TSX grammar, which also accepts plain JavaScript. Any other file uses the plain TypeScript grammar. It then collects the roots (names bound by imports and top-level declarations), records the module symbol, and walks each top-level statement with `item`.

The caller passes a `resolve` function. Given a relative import such as `./util/text`, it returns the module that file defines, or nothing when no such file is indexed. `Extractor::target` uses it to sort each import into three kinds: a repository module, an outside package, or unknown (a Node built-in, a missing file or a path alias). Repository imports go to the `Sink` as paths. Package imports go through `import_package` and record the names taken.

After the walk, any name listed in an `export { a, b }` clause is made public.

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
- [[Model Module]] — uses `Visibility`
- [[ParsedFile]] — uses `ParsedFile`

## Used by

- [[Scan Module]] — from `scan_repo`

## Gotchas

- Plain `.ts` needs its own grammar, because a cast like `<T>value` is not valid TSX. A `.ts` file with JSX would fail to parse cleanly.
- Only exported, top-level things are `Public`. Anything inside a function body is `Private`.
- A default import is recorded as the name `default` under the module's path.
- An import of a module file records a dependency on the whole module, even if the imported name is not an indexed symbol.
- A dotted call counts as a path only when it starts at a root. Otherwise it is a method call on an unknown receiver.
- A doc comment is the `/** */` block above, or failing that a run of `//` lines. A comment that trails code on its own line is not used.

## See also

- [[_index]]
- [[cw-parse]]
