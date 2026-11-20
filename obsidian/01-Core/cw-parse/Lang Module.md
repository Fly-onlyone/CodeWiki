---
tags: [core, cw-parse, module]
---

# Lang Module

> Plumbing shared by the Python, TypeScript and Java extractors: the fact sink, text helpers, the file-to-module mapping and import classification.

## What it does

Qualified names use `::` between segments in every language, so one resolver serves all of them: `pkg::util::shout` for Python's `pkg.util.shout`, `com::acme::Label` for Java's `com.acme.Label`, and `src::util::text::shout` for a TypeScript export of `src/util/text.ts`.

## Source

- `crates/cw-parse/src/lang.rs` — module `cw_parse::lang`

## How it works

1. `Extractor::qualify` in [[Java Module]] and `Extractor::import_from` in [[Python Module]], and 3 more, call `split_qualified`.
2. `Extractor::children` and `Extractor::enum_constant` in [[Java Module]], and 38 more, call `named`.
3. `Extractor::field` and `Extractor::member_names` in [[Java Module]], and 5 more, call `field_all`.
4. `Extractor::import` and `parse_java` in [[Java Module]], and 5 more, call `dotted`.
5. `Extractor::field` and `Extractor::signature` in [[Java Module]], and 7 more, call `collapse`.

11 more entry points: `marker_name`, `block_doc`, `dedent`, `starts_uppercase`, `is_generic_param`, `python_module`, and 5 more.

## Items

- `const NODE_BUILTINS: [&str; 34]` — modules built into Node.js, importable with or without the `node:` prefix.
- `pub(crate) fn named(node: Node) -> Vec<Node>` — named children of `node`, collected so the tree can be walked while the extractor is mutated.
- `pub(crate) fn field_all<'t>(node: Node<'t>, name: &str) -> Vec<Node<'t>>` — every child stored under the field `name`.
- `pub(crate) fn split_qualified(qualified: &str) -> Vec<String>` — internal function, returns `Vec<String>`; called by `Extractor::export`, `Extractor::import_from`, `Extractor::qualify`, `Extractor::target` and 2 more.
- `pub(crate) fn dotted(text: &str) -> Vec<String>` — segments of a dotted name such as `pkg.util.shout`.
- `pub(crate) fn collapse(raw: &str) -> String` — declaration text with whitespace collapsed and the body opener removed.
- `pub(crate) fn dedent(text: &str) -> String` — remove the indentation shared by the lines after the first.
- `pub(crate) fn block_doc(comment: &str) -> Option<String>` — text of a `/** ... */` comment with the markers and leading stars removed. Returns `None` for any other comment.
- `pub(crate) fn sanitize(segment: &str) -> String` — make a path component usable as a name segment.
- `pub(crate) fn starts_uppercase(name: &str) -> bool` — internal function, returns `bool`; called by `Extractor::invocation`, `Extractor::walk`.
- `pub(crate) fn is_generic_param(name: &str) -> bool` — single uppercase letters are almost always generic parameters.
- `pub(crate) fn marker_name(text: &str) -> String` — name of a decorator or annotation: the text after `@` up to its arguments.
- [[Sink]] — collects the facts of one file.
- `pub fn python_module(files: &HashSet<&str>, path: &str) -> (String, bool)` — module a Python file defines, and whether the file is a package (`__init__.py`). The name starts at the outermost directory that has an `__init__.py`, so it matches what `import` statements write. A file outside any package is named by its path from the repository root, minus a leading `src` or `lib`.
- `pub fn ts_module(path: &str) -> String` — module a TypeScript file defines: its path without the extension.
- `pub fn resolve_ts_import(files: &HashSet<&str>, from: &str, spec: &str) -> Option<String>` — file a relative TypeScript import specifier names, as written in `from`. Tries the specifier itself, then each source extension, then an `index` file in a directory of that name. Non-relative specifiers return `None`.
- `pub(crate) fn ts_package(spec: &str) -> Option<String>` — package a non-relative TypeScript import specifier names, or `None` when it is a Node.js builtin or a path alias whose target is not known.
- `pub(crate) fn java_default_package(path: &str) -> String` — module used for a Java file with no `package` declaration: its directory.
- `fn external(import: UseImport, package: String, package_len: usize) -> UseImport` — an import that leaves the repository: keep one segment naming the package, and what the import takes from it. The first `package_len` segments of the path name the package.
- `pub(crate) fn link_imports(parsed: &mut [(String, ParsedFile)])` — decide, for every Python and Java import, whether it names repository code, an outside package, or the language's own library. Imports of the last kind are dropped. Also records which Python package holds which module.

And 9 more items.

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
