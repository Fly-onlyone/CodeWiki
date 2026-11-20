---
tags: [core, cw-index, module]
---

# Search Module

> Full-text search over notes, symbols and files (SQLite FTS5, BM25).

## Source

- `crates/cw-index/src/search.rs` — module `cw_index::search`

## How it works

1. `Index::search` in [[cw-index]] calls `fts_query`.

## Items

- [[Document]] — one searchable unit.
- `pub struct Hit` — public struct; used by `Answer`, `Index::nearest`, `Index::search`, `nearest` and 2 more.
- `fn fts_query(text: &str) -> Option<String>` — turn free text into an FTS5 query: each word becomes a quoted prefix term, joined with OR so partial matches still rank. Quoting keeps user input from being read as FTS5 syntax.

## Used by

- [[Semantic Module]] — from `nearest`, `search`, `search_with`
- [[cw-index]] — from `nearest`, `search`
- [[Api Module]] — from `search`
- [[Query Module]] — from `Answer`

## See also

- [[_index]]
- [[cw-index]]
