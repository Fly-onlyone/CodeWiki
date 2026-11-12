---
tags: [core, cw-wiki, module]
---

# Semantic Module

> Semantic search: embeds search documents and blends vector matches with full-text matches.

## Source

- `crates/cw-wiki/src/semantic.rs` — module `cw_wiki::semantic`

## How it works

Main items:

- `embedder` — the embedder chosen by the environment, read once per process.
- `refresh` — `embed_documents` with the process embedder.
- `embed_documents` — bring the stored vectors of `kinds` in line with `documents`: embed new and changed documents, drop vectors of removed ones.
- `search_with` — search with full-text and vector matches merged by reciprocal rank fusion.
- `search` — `search_with` using the process embedder.
- `text_of` — function
- `MIN_SIMILARITY` — vector matches below this similarity are noise, not relatives.
- `OPENING_CHARS` — characters of a document's opening shown when full-text search gave no snippet.

## Depends on

- [[cw-index]] — uses `document_opening`, `nearest`, `search`, `store_vectors`
- [[cw-embed]] — uses `from_env`
- [[Error Module]] — uses `Result`
- [[Hash Module]] — uses `sha256_hex`
- [[Search Module]] — uses `Hit`
- [[Vectors Module]] — uses `VectorRow`
- [[Embedder]] — uses `Embedder`
- [[Document]] — uses `Document`

## Used by

- [[Service Module]] — from `index_notes`, `reindex`
- [[Api Module]] — from `search`

## See also

- [[_index]]
- [[cw-wiki]]
