---
tags: [core, cw-index, module]
---

# Vectors Module

> Embedding vectors for semantic search.

## Source

- `crates/cw-index/src/vectors.rs` — module `cw_index::vectors`

## How it works

Main items:

- `VectorRow` — one stored vector.
- `delete` — function
- `upsert` — function
- `from_blob` — function
- `to_blob` — function

## Depends on

- [[Error Module]] — uses `Result`
- [[Store Module]] — uses `db`

## Used by

- [[cw-index]] — from `nearest`, `prune_vectors`, `store_vectors`, `upsert_vectors`
- [[Run]] — from `store`

## See also

- [[_index]]
- [[cw-index]]
