---
tags: [moc, core, cw-index]
---

# Core / cw-index — Map of Content

> Notes for `cw-index`: 7 concepts.

## Notes

- [[cw-index]] — SQLite index: files, symbols, edges, metrics and full-text search for CodeWiki.
- [[Document]] — one searchable unit.
- [[Index]] — `Index` struct defined in `cw_index::store` with 19 methods.
- [[Schema Module]] — database schema.
- [[Search Module]] — Full-text search over notes, symbols and files (SQLite FTS5, BM25).
- [[Store Module]] — reads and writes of structural facts.
- [[Vectors Module]] — embedding vectors for semantic search.

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]
