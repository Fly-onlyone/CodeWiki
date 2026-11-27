---
tags: [core, cw-parse, module]
---

# Csharp Module

> C# symbol and fact extraction.

## What it does

Written against the node kinds and field names published in the grammar's own `node-types.json` (tree-sitter-c-sharp 0.24). No query files are used.

Mapping, in short. A namespace is a module, so the files of one namespace share one module, as the files of a Java package do. Symbols: a class and a record are `Class`, a struct and a record struct `Struct`, an interface `Interface`, an enum `Enum`, a delegate `TypeAlias`, a method, a constructor and an operator-free member function `Method`, a constant, a `static readonly` field and an enum member `Const`. Properties, events, indexers, operators and plain fields are no symbols: the types and values they mention count for their type, as a Java field does. Top-level statements are the function `Main` of their namespace.

## Source

- `crates/cw-parse/src/csharp.rs` — module `cw_parse::csharp`

## How it works

1. `scan_repo` in [[Scan Module]] calls `parse_csharp`. `parse_csharp` calls `Error::external`, then `Error::invalid` (both in [[Error]]), then `flat`, then `is_namespace`, then `namespace_segments`, then `csharp_default_namespace` in [[Lang Module]], and 1 more. `flat` calls `named` in [[Lang Module]]. `namespace_segments` calls `dotted` in [[Lang Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub fn parse_csharp(file: &str, source: &str) -> Result<ParsedFile>` — parse one C# source file. The module is the file's namespace, for example `Acme::Text` for `namespace Acme.Text`; a file without one is named by its directory. A `global using` is recorded with an empty scope: the scanner applies it to every namespace. Other imports are recorded as written; the scanner decides afterwards which of them name repository namespaces.
- `fn is_namespace(kind: &str) -> bool` — private function, returns `bool`; called by `parse_csharp`.
- `fn fielded<'t>(node: Node<'t>) -> Vec<(Option<&'t str>, Node<'t>)>` — the named children of `node` with the name of the field each one is in.
- `fn flat(node: Node) -> Vec<Node>` — the children of `node` that count as its items: the branches of a conditional compilation block stand where the block stands, and the other directives are left out.
- `fn namespace_segments(node: Node, src: &[u8]) -> Vec<String>` — segments of a namespace's name.
- `fn declarators(field: Node) -> Vec<Node>` — the declarators of a field.
- `fn xml_doc(text: &str) -> String` — the text of an XML documentation comment as plain paragraphs, with the `<param>`, `<returns>` and `<exception>` tags as the tag lines a Javadoc has, which the note writer already turns into list items.
- `fn xml_elements(text: &str) -> Vec<(String, String, String)>` — the top-level elements of `text`: tag, attribute text and inner text.
- `fn attribute(attributes: &str, name: &str) -> String` — the value of attribute `name` in the text of a tag.
- `fn short_cref(cref: &str) -> String` — A `cref` as the name a reader knows: `T:Acme.Oops` becomes `Oops`.
- `fn inline_xml(text: &str) -> String` — inline XML of a doc comment as Markdown text.

And 5 more private items with no comment.

## Depends on

- [[Lang Module]] — uses `block_doc`, `collapse`, `csharp_default_namespace`, `dotted`
- [[Sink]] — uses `Sink`, `between`, `inherits`, `new`
- [[Error]] — uses `external`, `invalid`
- [[Error Module]] — uses `Result`
- [[Visibility]] — uses `Visibility`
- [[ParsedFile]] — uses `ParsedFile`

## Used by

- [[Scan Module]] — from `scan_repo`

## See also

- [[_index]]
- [[cw-parse]]
