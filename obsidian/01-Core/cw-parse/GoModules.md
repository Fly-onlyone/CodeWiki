---
tags: [core, cw-parse]
---

# GoModules

> The Go modules of a repository: the folder of each `go.mod` and the module path it declares.

## Source

- `crates/cw-parse/src/gomod.rs` — definition of `GoModules`

## How it works

What the functions of this type call:

1. `package_of` calls `of`.

## Items

Declared as `pub struct GoModules`.

Members:

- `pub fn of(&self, file: &str) -> Option<&(String, String)>` — the module that holds `file`: the one whose folder is the nearest above it.
- `pub fn package_of(&self, file: &str) -> String` — the qualified name of the package that holds `file`: the module path then the folders below the module, every segment made safe. A file outside any module is named by its folder alone, or `""` at the root. This is what an import path of the package turns into, so an import names the package it finds.
- `pub fn folder_of(&self, import: &str) -> Option<String>` — the folder an import path names inside one of the modules, or `None` for a path outside all of them. The longest module path wins.

## Depends on

- [[Lang Module]] — uses `sanitize`

## Used by

- [[Lang Module]] — from `GoBuild`
- [[Scan Module]] — from `scan_repo`

## See also

- [[_index]]
- [[Gomod Module]]
