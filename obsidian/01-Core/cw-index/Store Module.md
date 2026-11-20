---
tags: [core, cw-index, module]
---

# Store Module

> Reads and writes of structural facts.

## Source

- `crates/cw-index/src/store.rs` — module `cw_index::store`

## How it works

Main items:

- [[Index]] — struct
- `NodeMetrics` — struct
- `db` — function
- `edge_from_row` — function
- `SYMBOL_COLUMNS` — const
- `symbol_from_row` — function

## Depends on

- [[Model Module]] — uses `Edge`, `parse`
- [[Error]] — uses `Error`, `external`
- [[Confidence]] — uses `parse`
- [[Symbol]] — uses `Symbol`
- [[SymbolKind]] — uses `parse`
- [[Index]] — uses `collect`

## Used by

- [[Index]] — from `collect`, `edges`, `edges_from`, `edges_to`
- [[cw-index]] — from `document_opening`, `documents`, `nearest`, `prune_vectors`
- [[Repo]] — from `at`, `build`, `new`
- [[Vectors Module]] — from `delete`, `upsert`
- [[Analyze Module]] — from `analyze`, `metrics_map`
- [[Generate Module]] — from `generate`

## See also

- [[_index]]
- [[cw-index]]
