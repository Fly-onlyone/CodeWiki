---
tags: [core, cw-parse]
---

# UseImport

> One name brought into scope by a `use` declaration.

## What it does

A `UseImport` is one name that a file brings into scope: a Rust `use`, a Python `import`, a TypeScript `import` or a Java `import`. It records the module the import sits in, the path as written, the name it binds (the alias), and whether it is a glob (`*`) or a public re-export. The resolver uses these to follow a bare name in a call back to the file that defines it.

## Source

- `crates/cw-parse/src/rust.rs` — definition of `UseImport`

## How it works

In Rust, `flatten_use` splits a `use a::{b, c as d}` into one `UseImport` for each name. In the other languages the extractor calls `Sink::import` for names that might be repository code, or `Sink::import_external` for a package that is certainly outside.

After parsing, `link_imports` in [[Lang Module]] checks each Python and Java import. Repository code stays as it is. A language's own library is dropped. An outside package is rewritten: `path` shrinks to the package name and `external` holds what the file takes from it, such as `adapters.HTTPAdapter` or `*`. Those rows later become the external-use facts in the index.

## Items

Declared as `pub struct UseImport`.

## Used by

- [[Lang Module]] — from `external`, `link_csharp`, `link_go`, `link_jvm`
- [[Sink]] — from `import`, `import_external`
- [[Rust Module]] — from `flatten_use`
- [[ParsedFile]] — from `ParsedFile`

## Gotchas

- An empty `alias` for a non-glob import binds nothing. It only states that the file depends on the path.
- For a glob import the alias is empty and `glob` is true.
- `external` is empty both for repository code and for an import of just the package. For Rust it stays empty even for outside crates, because `path` already says it.
- `public` marks a re-export (`pub use`). The resolver follows those, so a name can be reached through a module that only re-exports it.

## See also

- [[_index]]
- [[Rust Module]]
