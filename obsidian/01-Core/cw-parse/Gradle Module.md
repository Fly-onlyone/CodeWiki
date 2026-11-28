---
tags: [core, cw-parse, module]
---

# Gradle Module

> Gradle's version catalog, `gradle/libs.versions.toml`, read as TOML.

## What it does

Reads Gradle's version catalog, `gradle/libs.versions.toml`, so the scanner knows which JVM libraries a project asks for and at which version. A Kotlin or Java build script names a library by an alias such as `libs.kotlinx.coroutines.core`. The catalog is the only place that links that alias to a group, a name and a version. Only the default catalog, named `libs`, is read.

## Source

- `crates/cw-parse/src/gradle.rs` — module `cw_parse::gradle`

## How it works

`parse_version_catalog` parses the text as TOML, then walks the `[libraries]` table. Each entry has one of two shapes. A string `group:name:version` is split on colons. A table gives `module` (`group:name`), or `group` plus `name`. The version may be a plain string, a table holding `require`, `strictly` or `prefer` (first one found wins), or `version.ref`, which names a key of `[versions]`. The result is a list of `CatalogLibrary` values. `accessor` turns the alias into the script name by replacing `-` and `_` with dots under the `libs.` prefix. `jvm_requirement` in [[Manifests Module]] is the caller.

## Items

- `pub struct CatalogLibrary` — one entry of the `[libraries]` table.
- `pub fn parse_version_catalog(text: &str) -> Vec<CatalogLibrary>` — the libraries of a version catalog. A text that is no TOML gives none.

## Used by

- [[Manifests Module]] — from `jvm_requirement`

## Gotchas

- Text that is not valid TOML gives an empty list, with no error.
- An entry with no readable group and name is skipped.
- The version stays empty when the catalog gives none (a BOM decides it), when a `version.ref` names a missing key, or when the version table has none of the three known keys.
- Plugins in `[plugins]` are ignored.

## See also

- [[_index]]
- [[cw-parse]]
