---
tags: [core, cw-parse, module]
---

# Go Module

> Go symbol and fact extraction.

## What it does

Written against the node kinds and field names published in the grammar's own `node-types.json` (tree-sitter-go 0.25). No query files are used.

How Go maps onto the shared model:

- **A package is a module.** All `.go` files of one folder share one package; the caller names it (see `crate::gomod::GoModules::package_of`) so that the import path `example.com/app/internal/store` is the module `example_com::app::internal::store`. The module symbol's signature is `package <name>`, which is how an entry point is told from a library. An external test package (`package store_test` in a `_test.go` file) is a module of its own, `<package>_test`.
- **Functions** are `<package>::Name`; a **method** belongs to its receiver's type, value or pointer alike (`<package>::Type::Name`), so the type and its methods nest as a class and its methods do elsewhere.
- **Types.** A struct is a `SymbolKind::Struct`, an interface a `SymbolKind::Interface` whose methods are method symbols, and every other defined type and every alias a `SymbolKind::TypeAlias`. Fields are no symbols. Constants are `SymbolKind::Const` and package variables `SymbolKind::Static`.
- **Exported** (a name that starts with an upper-case letter) is `Visibility::Public`; any other name is visible to its package alone, which is `Visibility::Crate`.
- **Inheritance.** Go has none; the shared edge for it, "implements", is given to what stands closest: a struct that embeds a type, an interface that embeds an interface, and `var _ Iface = (*T)(nil)`, the idiom that states `T` implements `Iface`. A type that satisfies an interface by having its methods leaves no trace in the text and gets no edge.
- **Calls.** `pkg.Func()` through an import name is a path; `x.Method()` on the receiver of the enclosing method is a call on the receiver's type; `x.Method()` on anything else is a method call on a receiver of unknown type, as in Java. A name bound inside the function (parameter, `:=`, `var`, range variable) is never a package-level name.
- **Doc comments** are the `//` lines (or a `/* */` block) that end on the line above a declaration; compiler directives such as `//go:generate` are no documentation.
- **Generated code.** A file with a line `// Code generated ... DO NOT EDIT.` before its `package` clause is parsed like any other, so its symbols exist and are called, and each of its symbols carries the attribute `GENERATED_ATTR`, which keeps it out of the notes' rankings.

## Source

- `crates/cw-parse/src/go.rs` — module `cw_parse::go`

## How it works

1. `scan_repo` in [[Scan Module]] calls `parse_go`. `parse_go` calls `Error::external`, then `Error::invalid` (both in [[Error]]), then `named`, then `sanitize` (both in [[Lang Module]]), then `Sink::new` in [[Sink]], then `is_generated_marker`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const GENERATED_ATTR: &str` — the attribute of a symbol declared in a generated file.
- `const BUILTIN_FUNCTIONS: [&str; 16]` — names the compiler provides. A call to one is no call to repository code, and a type of this list names no type of the repository.
- `pub fn package_name(source: &str) -> Option<&str>` — the name in the `package` clause of a Go file, read without a parser, or `None` when the file has none before its first declaration.
- `pub fn is_exported(name: &str) -> bool` — true for a name Go exports: one that starts with an upper-case letter.
- `parse_go` — parse one Go source file. `module` is the qualified name of the package the file belongs to. `import_name` gives the name a package of the repository declares for itself, for an import path (`example.com/app/store` for `package db`); where it knows none, the last element of the path stands in.
- `fn is_generated_marker(comment: &str) -> bool` — true for the line Go tools read as "this file is generated": `// Code generated ... DO NOT EDIT.`.
- `struct Fun` — the names bound inside one function, which no package-level name of the same spelling can be: parameters, results, receivers, type parameters, `:=` and `var` names, range variables, local types.
- `fn is_directive(line: &str) -> bool` — true for a comment line that is a compiler or tool directive (`go:generate`, `nolint:errcheck`), which Go leaves out of documentation.
- `fn collect_locals(src: &[u8], node: Node, out: &mut HashSet<String>)` — every name bound anywhere inside `node`.
- `fn type_parameters_of(src: &[u8], ty: Node, out: &mut HashSet<String>)` — the type parameters a receiver type lists: `T` in `*Set[T]`.

And 3 more private items with no comment.

## Depends on

- [[Sink]] — uses `Sink`, `between`, `inherits`, `new`
- [[Lang Module]] — uses `collapse`, `field_all`, `named`, `sanitize`
- [[Error]] — uses `external`, `invalid`
- [[Error Module]] — uses `Result`
- [[Gomod Module]] — uses `guess_package_name`
- [[SymbolKind]] — uses `SymbolKind`
- [[Visibility]] — uses `Visibility`
- [[ParsedFile]] — uses `ParsedFile`

## Used by

- [[Scan Module]] — from `scan_repo`
- [[Health Module]] — from `dead_code`
- [[Repo]] — from `is_test`

## See also

- [[_index]]
- [[cw-parse]]
