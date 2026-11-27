---
tags: [core, cw-parse, module]
---

# Gradle Module

> Gradle's version catalog, `gradle/libs.versions.toml`, read as TOML.

## What it does

The format is documented at <https://docs.gradle.org/current/userguide/version_catalogs.html>. A library is written as a string `"group:name:version"`, or as a table with a `module` (`"group:name"`) or a `group` and a `name`, and a `version` that is a string, a table of `require`, `strictly` and `prefer`, or a `version.ref` naming a key of `[versions]`. A build script uses it by the accessor of its alias: `libs.kotlinx.coroutines.core` for `kotlinx-coroutines-core`. Only the default catalog, named `libs`, is read.

## Source

- `crates/cw-parse/src/gradle.rs` — module `cw_parse::gradle`

## How it works

1. `jvm_requirement` in [[Manifests Module]] calls `parse_version_catalog`.

## Items

- `pub struct CatalogLibrary` — one entry of the `[libraries]` table.
- `pub fn parse_version_catalog(text: &str) -> Vec<CatalogLibrary>` — the libraries of a version catalog. A text that is no TOML gives none.

## Used by

- [[Manifests Module]] — from `jvm_requirement`

## See also

- [[_index]]
- [[cw-parse]]
