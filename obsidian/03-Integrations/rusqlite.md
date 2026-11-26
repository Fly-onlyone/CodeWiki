---
tags: [integration]
---

# rusqlite

> External crate `rusqlite`, imported by 3 source files.

## What it does

`rusqlite` is the SQLite driver of the index. The index keeps every structural fact about a repository: files, symbols, edges between them, crates, search documents and embedding vectors. All of it sits in one SQLite database that only the `cw-index` crate touches. The `bundled` feature compiles SQLite into the program, so no system library is needed.

## Used for

Imported by 3 source files. Main users:

- [[Search Module]] — imports `params`
- [[Store Module]] — imports `Connection`, `OptionalExtension`, `Row`, `params`
- [[Vectors Module]] — imports `params`

## Configuration

- `crates/cw-index/Cargo.toml` — declares the dependency: version `0.43.0`, locked `0.43.0`, features `bundled`

## Wire-up

The `Index` struct owns one `Connection`. `Index::open` creates the parent folder, opens the file, sets WAL journaling and normal sync, then calls `init`. `open_in_memory` serves tests.

`init` reads `PRAGMA user_version`. If it differs from `SCHEMA_VERSION`, it drops every table and recreates them. Then it writes the version.

The [[Store Module]] writes facts and metrics, each inside one transaction. The [[Search Module]] runs full-text queries on an FTS5 table, with BM25 ranking where a title match weighs five times a body match. The [[Vectors Module]] stores vectors as little-endian f32 blobs and ranks them by dot product in Rust. All values are passed with the `params!` macro, never pasted into SQL.

## Auth mode

No sign-in. The database is a local file; there is no password or key.

## Gotchas

- A schema version change deletes the whole index. This is safe only because every fact can be rebuilt from the repository. Embedding vectors are rebuilt too, so a change costs a re-embed.
- `find_symbols` builds a LIKE pattern. It strips `%` from the input and escapes `_`, so a name with an underscore matches literally.
- Search text is turned into quoted prefix terms joined by OR, so user input is never read as FTS5 syntax. Text with no letters or digits returns no hits.
- `nearest` skips vectors whose length differs from the query. After changing embedder, old vectors give no results until re-embedded.
- Writes that replace data run in a transaction, so a failed write leaves the old data.

## See also

- [[_index]]
