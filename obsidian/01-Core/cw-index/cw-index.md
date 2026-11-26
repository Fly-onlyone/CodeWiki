---
tags: [core, cw-index]
---

# cw-index

> SQLite index: files, symbols, edges, metrics and full-text search for CodeWiki.

## What it does

Keeps what CodeWiki knows about a repository in one SQLite database: files, symbols, edges between them, graph metrics, full-text documents and embedding vectors. Other crates read it to answer questions fast, without parsing the code again.

The index is a cache. Everything in it can be rebuilt from the repository and the vault, so it is safe to delete at any time.

## Source

- `crates/cw-index/Cargo.toml` — package manifest
- `crates/cw-index/src/lib.rs` — crate root

## How it works

The crate root only declares four modules and re-exports their public types: `Index`, `NodeMetrics`, `Document`, `Hit` and `VectorRow`. The work is in the modules.

The [[Store Module]] holds `Index`. `Index::open` opens the database file, and `Index::open_in_memory` makes one for tests. Both check the schema version stored in the SQLite `user_version` setting. `replace_facts` writes a whole set of parsed facts. `replace_metrics` stores graph scores.

The [[Search Module]] owns the full-text table. `Index::replace_documents` loads documents into it. `Index::search` turns the user's words into a safe query with `fts_query` and returns ranked hits.

The [[Vectors Module]] stores embedding vectors. `store_vectors` and `upsert_vectors` write them, `prune_vectors` removes old ones, and `nearest` finds the closest ones. `vector_hashes` and `vector_kinds` let a caller see what is already embedded.

The [[Schema Module]] holds the table definitions and the version number.

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

- [[Semantic Module]] — from `nearest`, `search_with`
- [[Service Module]] — from `index_notes`, `reindex`
- [[Run]] — from `plan`, `store`
- [[codewiki]] — from `run`
- [[Embed Job Module]] — from `embed_repo`
- [[Query Module]] — from `answer`

## Gotchas

- A database with a different schema version is dropped and rebuilt, not migrated. Anyone who changes a table must raise `SCHEMA_VERSION`.
- Do not keep anything in the index that cannot be rebuilt. It is a cache and may be deleted.
- Vectors are stored under the name of the embedder that made them. Vectors from different embedders are never compared.

## See also

- [[_index]]
- [[Crate Dependency Flow]]
