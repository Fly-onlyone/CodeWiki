---
tags: [core, cw-index, module]
---

# Store Module

> Reads and writes of structural facts.

## Source

- `crates/cw-index/src/store.rs` — module `cw_index::store`

## How it works

1. `Index::documents` and `Index::replace_documents` in [[cw-index]], and 19 more, call `db`. `db` calls `Error::external` in [[Error]].

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
- [[Index]] — uses `collect`

## Used by

- [[Index]] — from `collect`, `edges`, `edges_from`, `edges_to`
- [[cw-index]] — from `document_opening`, `documents`, `nearest`, `prune_vectors`
- [[Repo]] — from `at`, `build`, `new`
- [[Vectors Module]] — from `delete`, `upsert`
- [[Analyze Module]] — from `analyze`, `metrics_map`
- [[Generate Module]] — from `generate`, `generate_with`

## See also

- [[_index]]
- [[cw-index]]
