---
tags: [core, cw-index, module]
---

# Schema Module

> Database schema.

## What it does

This module holds the SQL that defines the index database: a version number, the statements that create every table, and the statements that drop them. It has no logic. [[Index]] runs these strings when it opens a database.

## Source

- `crates/cw-index/src/schema.rs` — module `cw_index::schema`

## How it works

`SCHEMA` creates tables only if they are missing: `meta`, `files`, `symbols`, `edges`, `crates`, `external_uses`, `metrics`, `vectors`, and an FTS5 virtual table `documents`. Most tables are `WITHOUT ROWID` with a text primary key. `edges` has a composite key of source, destination and kind. `documents` is tokenized so that an underscore counts as part of a word, which keeps names like `sha256_hex` in one piece.

`DROP_ALL` removes every one of those tables. `SCHEMA_VERSION` is stored in the database as `user_version`. On open, a mismatch runs `DROP_ALL` and then `SCHEMA`.

## Items

- `pub const SCHEMA_VERSION: i64` — public constant; used by `Index::init`.
- `pub const SCHEMA: &str` — public constant; used by `Index::init`.
- `pub const DROP_ALL: &str` — public constant; used by `Index::init`.

## Used by

- [[Index]] — from `init`

## Gotchas

- Bump `SCHEMA_VERSION` on any change to `SCHEMA`. If you forget, an old database keeps the old tables, because `CREATE TABLE IF NOT EXISTS` skips them.
- Add every new table to `DROP_ALL` too. A table missing there survives a version bump with stale rows.
- There is no migration. A bump deletes all data, including search documents and vectors, and the index must be built again.

## See also

- [[_index]]
- [[cw-index]]
