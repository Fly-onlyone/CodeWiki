---
tags: [core, cw-index, module]
---

# Store Module

> Reads and writes of structural facts.

## What it does

This module is the core of the index database. It defines the [[Index]] struct that wraps one SQLite connection, and it reads and writes the structural facts of a repository: files, symbols, edges, crates, external uses and graph metrics. It also holds `NodeMetrics`, the per-node scores (PageRank, betweenness, strongly connected component, community, and layout x and y) that the graph views use. The other `cw-index` modules add their own methods to `Index`.

## Source

- `crates/cw-index/src/store.rs` — module `cw_index::store`

## How it works

`db` is a small helper. Given a context string such as "writing facts", it returns a closure that turns a `rusqlite::Error` into a CodeWiki `Error` carrying that context. Every query in the crate uses it.

`symbol_from_row` and `edge_from_row` turn a database row back into a `Symbol` or an `Edge`. Text columns like kind and confidence are parsed back into enums. The `attrs` column is a comma-joined string and is split again on read.

Writes follow one pattern: open a transaction, delete the old rows, insert the new ones, commit. A failure in the middle leaves the old data in place.

## Items

- `pub(crate) fn db(context: &'static str) -> impl Fn(rusqlite::Error) -> Error` — internal function, returns `impl Fn(rusqlite::Error) -> Error`; called by `Index::collect`, `Index::document_opening`, `Index::documents`, `Index::init` and 17 more.
- `pub struct NodeMetrics` — public struct; used by `Index::metrics`, `Index::replace_metrics`, `Repo::at`, `Repo::build` and 5 more.
- [[Index]] — struct

And 3 more private items with no comment.

## Depends on

- [[Model Module]] — uses `Edge`, `parse`
- [[Error]] — uses `Error`, `external`
- [[Confidence]] — uses `parse`
- [[Symbol]] — uses `Symbol`
- [[SymbolKind]] — uses `parse`
- [[Visibility]] — uses `parse`
- [[Index]] — uses `collect`

## Used by

- [[Index]] — from `collect`, `edges`, `edges_from`, `edges_to`
- [[cw-index]] — from `document_opening`, `documents`, `nearest`, `prune_vectors`
- [[Repo]] — from `at`, `build`, `new`
- [[Vectors Module]] — from `delete`, `upsert`
- [[Analyze Module]] — from `analyze`, `metrics_map`
- [[Generate Module]] — from `generate`, `generate_with`

## Gotchas

- `SYMBOL_COLUMNS` and the column order of the `symbols` table must match `symbol_from_row` by position. Change one, change all.
- Symbol attributes are joined with commas. An attribute that contains a comma would be split in two on read.
- Metrics store `x` and `y` as f64 in SQLite and read them back as f32, so tiny rounding is possible.
- `db` takes a `&'static str`, so a context message cannot be built at run time.

## See also

- [[_index]]
- [[cw-index]]
