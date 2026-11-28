---
tags: [core, cw-parse]
---

# GoModules

> The Go modules of a repository: the folder of each `go.mod` and the module path it declares.

## What it does

`GoModules` records each Go module of a repository as a pair: the folder holding its `go.mod` and the module path it declares. A repository can have several, for example in a `go.work` workspace. The Go parser and the scan use it to turn a file into a package name, and an import path into a folder.

## Source

- `crates/cw-parse/src/gomod.rs` — definition of `GoModules`

## How it works

`of` finds the module that holds a file: among modules whose folder is the root or contains the file's folder, the one with the longest folder wins, which is the nearest one above.

`package_of` builds the qualified package name for a file. It joins the module path and the folders below the module root, and runs every segment through `sanitize`, so `example.com/app/internal/store` becomes `example_com::app::internal::store`. A file outside any module is named from its own folder alone, and `""` at the root. This is the same form an import path turns into, so an import lands on the package it names.

`folder_of` goes the other way. Given an import path, it picks the module whose path is a prefix (longest wins) and returns the repository folder. It returns `None` for a path outside every module, such as a third-party import.

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

## Gotchas

- The longest match wins in both directions. A nested module with its own `go.mod` therefore takes its files away from the module above it.
- Matching is on whole path segments. A module `example.com/app` does not match `example.com/app2/x`.
- A repository without any `go.mod` gives an empty list, and every file then falls back to being named by its folder.

## See also

- [[_index]]
- [[Gomod Module]]
