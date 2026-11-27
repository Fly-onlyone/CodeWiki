---
tags: [core, cw-parse, module]
---

# Kotlin Module

> Kotlin symbol and fact extraction.

## What it does

Written against the node kinds and field names published in the grammar's own `node-types.json` (tree-sitter-kotlin-ng 1.1.0). No query files are used. Kotlin is mapped at the level Java is, and the two share their packages.

How Kotlin maps onto the shared model:

- **A package is a module**, written `package a.b.c` in the file, whatever folder the file stands in. A Java file of the same package names the same module, so a Java class and a Kotlin class of `com.acme.store` are siblings and resolve each other. A file with no `package` is in the default package of its folder, exactly as a Java file is.
- **Declarations.** A class, a data class, a sealed class and an inner class are `SymbolKind::Class`; an interface, a `fun interface` and an annotation class are `SymbolKind::Interface`; an `enum class` is a `SymbolKind::Enum` whose entries are `SymbolKind::Const`; a `typealias` is a `SymbolKind::TypeAlias`. An `object` is a class (its members are what Java calls static members), and a `companion object` is no symbol: its members belong to the class around it, so `Foo.create()` is `pkg::Foo::create` wherever `create` is written.
- **Top level.** A function at the top of a file is a `SymbolKind::Function` of its package; one inside a type is a `SymbolKind::Method`. A top-level property, and a property of an `object` or a companion, is a symbol too: `SymbolKind::Const` for a `val` and a `const val`, `SymbolKind::Static` for a `var`. A property of a class, the ones its primary constructor declares with `val` or `var` included, is no symbol, as a Java field is none: the type it names counts as a reference of its class.
- **Extension functions** (`fun String.shout()`) are functions or methods of the place they are written in, never of the receiver. They carry the attribute `EXTENSION_ATTR`, which lets a call on a receiver of unknown type (`name.shout()`) find them as it finds methods.
- **Visibility.** `public` (the default) is `Visibility::Public`, `internal` and `protected` are `Visibility::Crate`, `private` is `Visibility::Private`. Annotations, without their arguments, are the symbol's `attrs`. The documentation is the KDoc comment (`/** */`) directly above.
- **Inheritance.** The supertype list after `:`, a class with its constructor call and an interface alike, is the edge Java gives to `extends` and `implements`. A class that extends one of the Kotest spec styles carries `KOTEST_ATTR`: such a class is a test without an annotation.
- **Imports.** `import a.b.C`, `import a.b.*` and `import a.b.C as D`. A `Companion` segment of an import path is left out, since the companion is no symbol. The platform (`java.*`, `javax.*`, `kotlin.*`) is not an integration; `kotlinx.*` and every other outside root is a library.
- **Calls.** `foo()`, `Type.foo()`, `x.foo()` (a receiver of unknown type), `this.foo()`, a call through an import alias, a call with named arguments, a call with a trailing lambda (the lambda does not change the callee) and `Foo(1)`, a call whose name is a type: that is a construction, a reference to `Foo` and a call of its constructor. A bare name that is a member of an enclosing type is that member. The default imports (`kotlin.*`, `kotlin.collections.*`, `kotlin.io.*`...) are not imports of the file, so `listOf` or `println` is a bare name that no repository function answers to unless the package or an import of the file has one: that is Kotlin's own rule.
- **Not followed.** Names bound inside a function (a lowercase name in an expression is a variable, as in Java), inherited members, a call through a typed receiver, and a top-level function called from Java as `UtilsKt.shout()`.
- **A grammar limit.** `fun interface I { ... }` makes the grammar lose the rest of the file, so the parser hands the grammar a copy of the source in which that `fun` is blanked out (same length, so every position still stands). Other constructs the grammar cannot read make the file report syntax errors; what could be read is still extracted, and the members of an error node are taken as members of the scope they stand in.

## Source

- `crates/cw-parse/src/kotlin.rs` — module `cw_parse::kotlin`

## How it works

1. `scan_repo` in [[Scan Module]] calls `parse_kotlin`. `parse_kotlin` calls `blank_fun_interfaces`, then `Error::external`, then `Error::invalid` (both in [[Error]]), then `named`, then `dotted`, then `sanitize` (all in [[Lang Module]]), and 2 more.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

## See also

- [[_index]]
- [[cw-parse]]
