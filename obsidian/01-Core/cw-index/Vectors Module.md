---
tags: [core, cw-index, module]
---

# Vectors Module

> Embedding vectors for semantic search.

## What it does

The index stores vectors and compares them; it does not know how they were made. Each row carries a hash of the text and embedder it came from, so only changed documents are embedded again.

## Source

- `crates/cw-index/src/vectors.rs` — module `cw_index::vectors`

## How it works

1. `Index::nearest` in [[cw-index]] calls `from_blob`.
2. `Index::prune_vectors` and `Index::store_vectors` in [[cw-index]] call `delete`. `delete` calls `db` in [[Store Module]].
3. `Index::store_vectors` and `Index::upsert_vectors` in [[cw-index]] call `upsert`. `upsert` calls `db` in [[Store Module]], then `to_blob`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub struct VectorRow` — one stored vector. `kind` and `key` match the search document it embeds.
- `fn to_blob(vector: &[f32]) -> Vec<u8>` — private function, returns `Vec<u8>`; called by `upsert`.
- `fn from_blob(blob: &[u8]) -> Vec<f32>` — private function, returns `Vec<f32>`; called by `Index::nearest`.
- `fn delete(tx: &rusqlite::Transaction<'_>, ids: &[(String, String)]) -> Result<()>` — private function, returns `Result<()>`; called by `Index::prune_vectors`, `Index::store_vectors`.
- `fn upsert(tx: &rusqlite::Transaction<'_>, rows: &[VectorRow]) -> Result<()>` — private function, returns `Result<()>`; called by `Index::store_vectors`, `Index::upsert_vectors`.

## Depends on

- [[Error Module]] — uses `Result`
- [[Store Module]] — uses `db`

## Used by

- [[cw-index]] — from `nearest`, `prune_vectors`, `store_vectors`, `upsert_vectors`
- [[Run]] — from `store`

## See also

- [[_index]]
- [[cw-index]]
