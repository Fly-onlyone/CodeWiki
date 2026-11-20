---
tags: [core, cw-index]
---

# cw-index

> SQLite index: files, symbols, edges, metrics and full-text search for CodeWiki.

## What it does

SQLite index for CodeWiki.

The index is a cache: everything in it can be rebuilt from the repository and the vault. It is safe to delete at any time.

## Source

- `crates/cw-index/Cargo.toml` — package manifest
- `crates/cw-index/src/lib.rs` — crate root

## How it works

Builds a library.

What the functions of this crate call:

1. `Index::replace_documents` calls `db` in [[Store Module]].
2. `Index::documents` calls `db` in [[Store Module]].
3. `Index::search` calls `fts_query` in [[Search Module]], then `db` in [[Store Module]].
4. `Index::vector_hashes` calls `db` in [[Store Module]].
5. `Index::vector_kinds` calls `db` in [[Store Module]].

5 more entry points: `Index::store_vectors`, `Index::upsert_vectors`, `Index::prune_vectors`, `Index::nearest`, `Index::document_opening`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Modules:

- [[Schema Module]] — database schema.
- [[Search Module]] — Full-text search over notes, symbols and files (SQLite FTS5, BM25).
- [[Store Module]] — reads and writes of structural facts.
- [[Vectors Module]] — embedding vectors for semantic search.

## Depends on

- [[Vectors Module]] — uses `VectorRow`, `delete`, `from_blob`, `upsert`
- [[Search Module]] — uses `Hit`, `fts_query`
- [[Error Module]] — uses `Result`
- [[Store Module]] — uses `db`
- [[Document]] — uses `Document`

## Used by

- [[Service Module]] — from `index_notes`, `reembed`, `reindex`
- [[Semantic Module]] — from `nearest`, `search_with`
- [[Run]] — from `plan`, `store`
- [[codewiki]] — from `run`
- [[cw-wiki]] — from `the_documents_of_an_index_are_embedded_again_in_steps`
- [[Models Module]] — from `embed_repo`
- [[Query Module]] — from `answer`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
