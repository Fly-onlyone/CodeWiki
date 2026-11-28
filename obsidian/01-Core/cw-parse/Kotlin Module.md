---
tags: [core, cw-parse, module]
---

# Kotlin Module

> Kotlin symbol and fact extraction.

## What it does

Turns one Kotlin source file into the shared model of symbols, imports, calls and inheritance edges that the scanner stores. Kotlin is mapped at the level Java is, and the two share their packages. A Java class and a Kotlin class of the same package are siblings and resolve each other. The mapping is written against the node kinds in the grammar's own `node-types.json` (tree-sitter-kotlin-ng 1.1.0). No query files are used.

In short: a package is a module, whatever folder the file is in. Classes, interfaces, enums, enum entries and type aliases become symbols. An `object` is a class, and a `companion object` is no symbol: its members belong to the class around it. Top-level functions and properties belong to the package. Extension functions belong to the place they are written, never to the receiver, and carry `EXTENSION_ATTR` so a call on an unknown receiver can find them.

## Source

- `crates/cw-parse/src/kotlin.rs` — module `cw_parse::kotlin`

## How it works

`parse_kotlin` first calls `blank_fun_interfaces`, then loads the grammar and parses the text. The `package` header gives the module name. A file with no header is named by its folder, as Java does. The function writes a module symbol, then walks the top-level nodes. Imports go to `import`. Everything else goes to `declaration`, which hands out to the type, function, property and alias handlers.

A `Level` value records where a declaration stands (file, type body, object), because that decides whether a function is a function or a method and whether a property is a symbol. Calls are collected while walking: plain calls, `Type.foo()`, calls through an import alias, and `Foo(1)`, which counts as a reference to `Foo` plus a call of its constructor. A class that extends a Kotest spec style (the `KOTEST_SPECS` list) gets `KOTEST_ATTR`, so it counts as a test with no annotation. Imports are stored as written, and the scanner decides later which name repository packages.

## Items

- `pub const EXTENSION_ATTR: &str` — the attribute of an extension function or property.
- `pub const KOTEST_ATTR: &str` — the attribute of a class that extends one of the Kotest spec styles.
- `const KOTEST_SPECS: [&str; 10]` — the spec styles of Kotest. A class that extends one is a test, though nothing on it says so.
- `pub fn parse_kotlin(file: &str, source: &str) -> Result<ParsedFile>` — parse one Kotlin source file (`.kt`, or a `.kts` script that is no Gradle build file). The module is the file's `package`, for example `com::acme::text`; a file without one is named by its directory. Imports are recorded as written; the scanner decides afterwards which of them name repository packages.
- `fn blank_fun_interfaces(source: &str) -> (Cow<'_, str>, Vec<usize>)` — the source with the `fun` of every `fun interface` blanked out, and where those keywords stand. The grammar reads a `fun interface` with a body as a function and loses the rest of the file; without the keyword it reads an ordinary interface. Blank bytes keep every position.
- `enum Level` — where a declaration stands, which decides what a function and a property in it are.
- `fn first_identifier(node: Node) -> Option<Node>` — the first identifier among the children of `node`: the name a declaration binds.
- `fn variables(property: Node) -> Vec<Node>` — the `variable_declaration` nodes of a property: one, or the several of a destructuring declaration.

And 3 more private items with no comment.

## Depends on

- [[Lang Module]] — uses `block_doc`, `collapse`, `dotted`, `java_default_package`
- [[Sink]] — uses `Sink`, `between`, `inherits`, `new`
- [[Error]] — uses `external`, `invalid`
- [[Error Module]] — uses `Result`
- [[Visibility]] — uses `Visibility`
- [[ParsedFile]] — uses `ParsedFile`

## Used by

- [[Health Module]] — from `dead_code`, `is_test_symbol`
- [[Scan Module]] — from `scan_repo`
- [[Testmap Module]] — from `is_jvm_test`
- [[Resolver]] — from `resolve`

## Gotchas

- The grammar loses the rest of the file after a `fun interface` with a body. `blank_fun_interfaces` blanks the `fun` word with spaces of the same length, so every position still stands.
- Other constructs the grammar cannot read give syntax errors. What could be read is still extracted, and members of an error node count as members of the scope they stand in.
- Default imports such as `kotlin.*` are not imports of the file. A bare `listOf` resolves to nothing unless the package or an import has it. That is Kotlin's own rule.
- Not followed: names bound inside a function, inherited members, calls through a typed receiver, and a top-level function called from Java as `UtilsKt.shout()`.
- A property of a class is no symbol, as a Java field is none. Only the type it names counts as a reference.

## See also

- [[_index]]
- [[cw-parse]]
