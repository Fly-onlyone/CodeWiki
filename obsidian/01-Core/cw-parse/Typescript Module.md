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

`parse_typescript` calls `parse_typescript_with` with an alias check that always says no. The function picks a grammar by file extension. `.tsx`, `.js`, `.jsx`, `.mjs` and `.cjs` use the TSX grammar, which also accepts plain JavaScript. Any other file uses the plain TypeScript grammar. It then collects the roots (names bound by imports and top-level declarations), records the module symbol, and walks each top-level statement with `item`.

The caller passes two functions. `resolve` takes an import specifier such as `./util/text` and returns the module that file defines, or nothing when no such file is indexed. `is_alias` says that a specifier matches a path alias of the repository's tsconfig. `Extractor::target` uses both to sort each import into three kinds: a repository module, an outside package, or unknown. The order is: `resolve` first, then the alias check (unknown, so it is not an outside package), then `ts_package` from [[Lang Module]], which names the package or returns nothing for a Node built-in or an unknown alias. Repository imports go to the [[Sink]] as paths. Package imports go through `import_package` and record the names taken.

After the walk, any name listed in an `export { a, b }` clause is made public.

## Items

- `parse_typescript` — parse one TypeScript or TSX source file. `module` is the qualified module the file defines, for example `src::util::text`. `resolve` maps a relative import specifier written in this file to the module it names, or `None` when no such file is indexed.
- `parse_typescript_with` — as `parse_typescript`, with `is_alias` saying that a specifier is an alias of the repository (a `paths` pattern of its tsconfig matches): such an import is the repository's own, so it is no outside package even when no file was found for it.
- `enum Target` — what an import specifier names.
- `fn pattern_names(node: Node, src: &[u8], out: &mut HashSet<String>)` — names bound by a declaration or parameter pattern.
- `fn bindings(node: Node, src: &[u8], out: &mut HashSet<String>)` — names bound anywhere in a function scope, not descending into nested functions and classes.

And 4 more private items with no comment.

## Depends on

- [[Lang Module]] — uses `block_doc`, `collapse`, `dotted`, `field_all`
- [[Sink]] — uses `Sink`, `between`, `import_external`, `inherits`
- [[Error]] — uses `external`, `invalid`
- [[Error Module]] — uses `Result`
- [[Visibility]] — uses `Visibility`
- [[ParsedFile]] — uses `ParsedFile`

## Used by

- [[Scan Module]] — from `scan_repo`

## Gotchas

- Plain `.ts` needs its own grammar, because a cast like `<T>value` is not valid TSX. A `.ts` file with JSX would fail to parse cleanly.
- Only exported, top-level things are `Public`. Anything inside a function body is `Private`.
- A default import is recorded as the name `default` under the module's path or package.
- An import of a module file records a dependency on the whole module, even if the imported name is not an indexed symbol.
- An import that matches a tsconfig alias but names no indexed file is not an outside package. It is dropped as unknown.
- A dotted call counts as a path only when it starts at a root. Otherwise it is a method call on an unknown receiver.
- A doc comment is the `/** */` block above, or failing that a run of `//` lines. A comment that trails code on its own line is not used.

## See also

- [[_index]]
- [[cw-parse]]
