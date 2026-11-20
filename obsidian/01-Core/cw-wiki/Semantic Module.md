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
- [[Run]] — one pass that brings the stored vectors of some kinds in line with their documents, a batch at a time.
- `embedder` — the embedder this process uses; `None` when semantic search is off.
- `Progress` — how far a run is.
- `behind` — whether documents of `kinds` still wait for a vector of the embedder `name`: another embedder made the stored ones, or a run was cut short.
- `refresh` — `embed_documents_with` with the process embedder, within the inline budget.
- `embed_documents_with` — bring the stored vectors of `kinds` in line with `documents`: embed new and changed documents, drop vectors of removed ones.
- `configure_in` — `configure` with the data directory that holds the built-in models.

## Depends on

- [[Run]] — uses `finish`, `is_done`, `plan`, `progress`
- [[cw-index]] — uses `document_opening`, `nearest`, `search`, `vector_kinds`
- [[Slot]] — uses `Slot`, `configure`, `configure_in`, `new`
- [[Index]] — uses `Index`, `meta`
- [[Error Module]] — uses `Result`
- [[Search Module]] — uses `Hit`
- [[Resolved]] — uses `Resolved`
- [[Embedder]] — uses `Embedder`

## Used by

- [[Models Module]] — from `after_update`, `embed_repo`, `resume`, `switch`
- [[Run]] — from `Run`, `finish`, `plan`, `progress`
- [[Service Module]] — from `index_notes`, `reembed`, `reindex`
- [[cw-server]] — from `serve`, `waits`
- [[Slot]] — from `configure_in`, `get`
- [[codewiki]] — from `run`
- [[cw-wiki]] — from `the_documents_of_an_index_are_embedded_again_in_steps`
- [[Serve Module]] — from `run`

## See also

- [[_index]]
- [[cw-wiki]]
