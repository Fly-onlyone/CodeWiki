---
tags: [core, cw-wiki, module]
---

# Semantic Module

> Semantic search: embeds search documents and blends vector matches with full-text matches.

## Source

- `crates/cw-wiki/src/semantic.rs` — module `cw_wiki::semantic`

## How it works

Main items:

- [[Slot]] — the place that holds the embedder in use.
- `embedder` — the embedder this process uses; `None` when semantic search is off.
- `refresh` — `embed_documents` with the process embedder.
- `embed_documents` — bring the stored vectors of `kinds` in line with `documents`: embed new and changed documents, drop vectors of removed ones.
- `configure` — use the embedder the settings in effect name.
- `search_with` — search with full-text and vector matches merged by reciprocal rank fusion.
- `search` — `search_with` using the process embedder.
- `EMBEDDER` — static

## Depends on

- [[cw-index]] — uses `document_opening`, `nearest`, `search`, `store_vectors`
- [[Index]] — uses `Index`, `meta`, `set_meta`
- [[Search Module]] — uses `Document`, `Hit`
- [[Slot]] — uses `Slot`, `configure`
- [[Error Module]] — uses `Result`
- [[Hash Module]] — uses `sha256_hex`
- [[Vectors Module]] — uses `VectorRow`
- [[Resolved]] — uses `Resolved`

## Used by

- [[Service Module]] — from `index_notes`, `reindex`
- [[Slot]] — from `configure`, `get`
- [[codewiki]] — from `run`
- [[cw-server]] — from `serve`
- [[Serve Module]] — from `run`
- [[Api Module]] — from `search`
- [[Settings Module (cw-server)]] — from `change`

## See also

- [[_index]]
- [[cw-wiki]]
