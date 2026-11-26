---
tags: [core, cw-wiki]
---

# Unit

> One top-level part of the repository.

## What it does

Describes one top-level part of a repository: a Rust crate, a Python package, a JavaScript or TypeScript package, or a Java build module. All units have the same fields, so the planner and the renderer need no special case per language. A unit gets one crate note and one folder under `01-Core`.

## Source

- `crates/cw-wiki/src/units.rs` — definition of `Unit`

## How it works

[[Units Module]] fills the fields in `discover`. The `name` is a stable key from which note ids are made. The `package` is the name shown to readers. The `slug` is the safe folder and tag name. `root` is the module every other module of the unit nests in, and `strip` says how many leading path segments carry no meaning inside the unit. `manifests` lists build files, the main one first, with their text. `entries` lists the symbols where execution starts. `is_rust` tells the language. `declares` returns the manifests that mention a package name as a whole word, ignoring case. [[Units]] holds the list.

## Items

Declared as `pub struct Unit`.

Members:

- `pub fn is_rust(&self) -> bool` — public method, returns `bool`; called by `RenderCtx::render_crate`, `Units::all_rust`.
- `pub fn declares(&self, package: &str) -> Vec<&str>` — true when a build file of the unit mentions `package` as a whole word.

## Depends on

- [[Units Module]] — uses `Entry`, `Manifest`
- [[Language]] — uses `Language`

## Used by

- [[RenderCtx]] — from `render_crate`, `render_operations`, `render_type`, `summary`
- [[Manifests Module]] — from `cargo`, `declared`, `other`
- [[Repo]] — from `module_count`, `top_modules`, `unit_of`
- [[Units]] — from `Units`, `all_rust`, `by_name`
- [[Plan Module]] — from `module_dir`, `plan`
- [[Units Module]] — from `discover`

## Gotchas

- `root` can be `None`. The planner then picks the largest top modules to stand for the unit.
- `declares` is a text match, not a parse. It cannot tell a dependency from a comment that names it.
- `manifests` text is empty when the file was not read.

## See also

- [[_index]]
- [[Units Module]]
