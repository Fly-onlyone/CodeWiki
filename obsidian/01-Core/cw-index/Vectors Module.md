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
- `from_blob` — function
- `to_blob` — function

## Used by

- [[cw-index]] — from `nearest`, `store_vectors`
- [[Semantic Module]] — from `embed_documents`

## See also

- [[_index]]
- [[cw-index]]
