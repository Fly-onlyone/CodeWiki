---
tags: [core, cw-index, module]
---

# Schema Module

> Database schema.

## What it does

Bump `SCHEMA_VERSION` on any change; an index with a different version is dropped and rebuilt rather than migrated.

## Source

- `crates/cw-index/src/schema.rs` — module `cw_index::schema`

## Items

- `pub const SCHEMA_VERSION: i64` — public constant; used by `Index::init`.
- `pub const SCHEMA: &str` — public constant; used by `Index::init`.
- `pub const DROP_ALL: &str` — public constant; used by `Index::init`.

## Used by

- [[Index]] — from `init`

## See also

- [[_index]]
- [[cw-index]]
