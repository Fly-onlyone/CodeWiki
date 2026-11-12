---
tags: [core, cw-index]
---

# cw-index

> SQLite index: files, symbols, edges, metrics and full-text search for CodeWiki.

## Source

- `crates/cw-index/Cargo.toml` — package manifest
- `crates/cw-index/src/lib.rs` — crate root

## How it works

Builds a library.

Modules:

- [[Schema Module]] — database schema.
- [[Search Module]] — Full-text search over notes, symbols and files (SQLite FTS5, BM25).
- [[Store Module]] — reads and writes of structural facts.
- [[Vectors Module]] — embedding vectors for semantic search.

## Depends on

- [[Vectors Module]] — uses `VectorRow`, `from_blob`, `to_blob`
- [[Search Module]] — uses `Hit`, `fts_query`
- [[Error Module]] — uses `Result`
- [[Store Module]] — uses `db`
- [[Document]] — uses `Document`

## Used by

- [[Semantic Module]] — from `embed_documents`, `search_with`
- [[Service Module]] — from `index_notes`, `reindex`
- [[codewiki]] — from `run`
- [[Query Module]] — from `answer`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
