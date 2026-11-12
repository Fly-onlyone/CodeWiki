---
tags: [core, cw-embed]
---

# cw-embed

> Text embeddings for CodeWiki's semantic search: a built-in local embedder and HTTP providers.

## Source

- `crates/cw-embed/Cargo.toml` — package manifest
- `crates/cw-embed/src/lib.rs` — crate root

## How it works

Builds a library.

Defined at the crate root:

- [[Embedder]] — trait
- [[LocalEmbedder]] — the built-in embedder: signed feature hashing, no model and no network.
- `normalize` — scale `vector` to unit length.
- `OllamaEmbedder` — A model served by Ollama on this machine or another.
- `OpenAiEmbedder` — any endpoint that speaks the OpenAI embeddings API.
- `from_env` — the embedder chosen by the environment; `None` turns semantic search off.
- `from_config` — the embedder for a provider name.
- `stem` — strip a common English ending so `computes`, `computed` and `computing` count as one word.

## Depends on

- [[Error]] — uses `external`, `invalid`
- [[cw-llm]] — uses `header`
- [[Error Module]] — uses `Result`
- [[Embedder]] — uses `Embedder`
- [[LocalEmbedder]] — uses `default`

## Used by

- [[LocalEmbedder]] — from `add`, `embed_one`
- [[Semantic Module]] — from `embedder`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
