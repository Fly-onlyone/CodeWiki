---
tags: [core, cw-index, module]
---

# Search Module

> Full-text search over notes, symbols and files (SQLite FTS5, BM25).

## Source

- `crates/cw-index/src/search.rs` — module `cw_index::search`

## How it works

Main items:

- [[Document]] — one searchable unit.
- `Hit` — struct
- `fts_query` — turn free text into an FTS5 query: each word becomes a quoted prefix term, joined with OR so partial matches still rank.

## Used by

- [[cw-index]] — from `nearest`, `search`
- [[Semantic Module]] — from `search`, `search_with`
- [[Api Module]] — from `search`
- [[Query Module]] — from `Answer`

## See also

- [[_index]]
- [[cw-index]]
