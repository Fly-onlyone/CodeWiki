---
tags: [core, cw-index, module]
---

# Vectors Module

> Embedding vectors for semantic search.

## What it does

This module stores embedding vectors (lists of numbers that stand for the meaning of a text) and finds the ones closest to a query. It supports semantic search next to the keyword search of [[Search Module]]. The index only stores and compares vectors. Another crate makes them. Each row keeps a hash of the text and of the embedder that made it, so a caller can see which documents changed and embed only those.

## Source

- `crates/cw-index/src/vectors.rs` — module `cw_index::vectors`

## How it works

A vector is saved as raw little-endian f32 bytes in a BLOB column. `to_blob` writes them and `from_blob` reads them.

`vector_hashes` returns the stored hash for each `(kind, key)`. A caller compares those with fresh hashes to find what needs embedding.

There are three ways to write. `store_vectors` inserts rows and deletes every vector of the given kinds that is not in the `keep` set, in one transaction. `upsert_vectors` only inserts or replaces, so an embedding run in batches keeps its finished batches if it stops early. `prune_vectors` only deletes the stale ones and returns the count.

`nearest` reads the vectors of a kind (or all), takes the dot product with the query, sorts best first with the key as tie-break, and cuts to the limit. `document_opening` returns the first characters of a document body with whitespace squeezed, for hits that have no snippet.

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

## Gotchas

- Scores are plain dot products. They are cosine similarity only if both the stored vectors and the query are unit length.
- A stored vector whose length differs from the query is skipped without error. After changing the embedder, old vectors give no hits until they are replaced.
- Pruning is limited to the kinds you name. Vectors of other kinds survive even when they are not in `keep`.
- `nearest` loads every matching row and scores it in memory. It scans all vectors on each call.

## See also

- [[_index]]
- [[cw-index]]
