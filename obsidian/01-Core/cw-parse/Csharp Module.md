---
tags: [core, cw-parse, module]
---

# Csharp Module

> C# symbol and fact extraction.

## What it does

Turns one C# file into symbols and facts for the shared model. A namespace is a module, so all files of one namespace share one module, like a Java package. Classes and records become classes, structs and record structs become structs, interfaces and enums keep their kinds, and delegates become type aliases. Methods and constructors are methods. Constants, `static readonly` fields and enum members are constants. Properties, events, indexers, operators and plain fields are not symbols; the types they mention count for their owner type. Top-level statements become a function `Main` of their namespace.

It works from the grammar's own node kinds and field names, with no query files.

## Source

- `crates/cw-parse/src/csharp.rs` — module `cw_parse::csharp`

## How it works

`parse_csharp` loads the tree-sitter C# grammar and parses the file. `flat` lists the top-level items and treats the branches of `#if` blocks as if they stood in the code; other directives are dropped. The first `namespace_declaration` or file-scoped namespace gives the module name through `namespace_segments`. Without one, `csharp_default_namespace` names the module from the file's folder.

An `Extractor` writing into a `Sink` then walks the items in a `Ctx` that carries the scope, namespace, enclosing types and the default access level for members. `Modifiers::visibility` maps `public` to public, `private` and `file` to private, and `internal` and `protected` to crate level. With no keyword the context default applies, which starts at crate level.

XML doc comments are turned into plain paragraphs by `xml_doc`, with `param`, `returns` and `exception` tags as tag lines like Javadoc. A `cref` is shortened to a readable name by `short_cref`.

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

## Gotchas

- A `global using` is recorded with an empty scope. The scan applies it to every namespace. Other imports are stored as written, and the scan decides later which ones name repository namespaces.
- Members with no modifier get the default of the enclosing context, so the result depends on what the member sits in.
- If a file has a namespace, the module symbol comes from the namespace itself. Only a file with no namespace gets an explicit module symbol.
- Because properties and fields are not symbols, a dead-code or usage view will not list them.

## See also

- [[_index]]
- [[cw-parse]]
