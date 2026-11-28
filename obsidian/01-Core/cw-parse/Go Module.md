---
tags: [core, cw-parse, module]
---

# Go Module

> Go symbol and fact extraction.

## What it does

Turns one Go file into symbols and facts for the shared model, using the tree-sitter Go grammar. A Go package is a module: all files of one folder share it, and the caller supplies its qualified name (see [[GoModules]]). Functions are `package::Name`. Methods belong to their receiver's type, whether the receiver is a value or a pointer. Structs, interfaces and every other type are separate kinds, constants are constants and package variables are statics. Fields are not symbols.

Go has no inheritance, so the shared "implements" edge is given to the closest thing the text states: an embedded type, an embedded interface, and the idiom `var _ Iface = (*T)(nil)`. A type that satisfies an interface only by having the methods leaves no edge.

## Source

- `crates/cw-parse/src/go.rs` — module `cw_parse::go`

## How it works

`parse_go` parses the file and reads the `package` clause. If the file is `_test.go` and the package ends in `_test`, the module becomes `<module>_test`, since an external test package is its own package. It adds the module symbol with the signature `package <name>`, which is how an entry point is told from a library.

It then reads imports first, and walks functions, methods, type declarations and const or var blocks. For calls it uses `Fun`, the set of names bound inside one function (parameters, `:=`, `var`, range variables, local types). A name bound locally is never treated as a package-level name. `pkg.Func()` through an import is a path call, `x.Method()` on the receiver is a call on the receiver's type, and any other `x.Method()` is a call on an unknown receiver. The `import_name` callback gives the real name a repository package declares for itself, so that `example.com/app/store` for `package db` resolves to `db`.

A file is generated when a comment before its `package` clause has a line starting `// Code generated` and ending `DO NOT EDIT.`. Its symbols are still extracted but carry the `generated` attribute.

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

## Gotchas

- Export rule: a name starting with an upper-case letter is public. Any other is crate level, because it is visible to its package only. There is no private level.
- Compiler directives like `//go:generate` or `//nolint` are not part of documentation.
- Builtin names (`len`, `make` and others in `BUILTIN_FUNCTIONS`) never count as calls to repository code.
- Generated files are kept in the index but marked, so rankings and notes skip them.
- Interface satisfaction by method set is invisible, so a call through an interface has no implements edge to follow.

## See also

- [[_index]]
- [[cw-parse]]
