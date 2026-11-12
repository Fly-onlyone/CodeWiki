---
tags: [core, cw-index, module]
---

# Schema Module

> Database schema.

## Source

- `crates/cw-index/src/schema.rs` — module `cw_index::schema`

## How it works

Bump `SCHEMA_VERSION` on any change; an index with a different version is dropped and rebuilt rather than migrated.

Main items:

- `DROP_ALL` — const
- `SCHEMA` — const
- `SCHEMA_VERSION` — const

## Used by

- [[Index]] — from `init`

## See also

- [[_index]]
- [[cw-index]]
