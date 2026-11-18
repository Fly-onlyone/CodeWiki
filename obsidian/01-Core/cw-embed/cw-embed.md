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
- [[OpenAiEmbedder]] — any endpoint that speaks the OpenAI embeddings API.
- [[OllamaEmbedder]] — A model served by Ollama on this machine or another.
- `normalize` — scale `vector` to unit length.
- `from_config` — the embedder for a provider name.
- `from_settings` — the embedder chosen by the settings in effect: the environment over the settings file.
- `PROVIDERS` — the embedders a user can choose, in the order a page lists them.

## Depends on

- [[Error]] — uses `external`, `invalid`
- [[cw-llm]] — uses `header`
- [[Error Module]] — uses `Result`
- [[Logs Module]] — uses `flush`
- [[Backend]] — uses `status`
- [[Resolved]] — uses `Resolved`
- [[Embedder]] — uses `Embedder`
- [[LocalEmbedder]] — uses `default`

## Used by

- [[Settings Module (cw-server)]] — from `checked`, `describe`, `test_embedder`
- [[OllamaEmbedder]] — from `embed`, `embed_within`, `probe`
- [[OpenAiEmbedder]] — from `embed`, `embed_within`, `probe`
- [[LocalEmbedder]] — from `add`, `embed_one`
- [[Slot]] — from `configure`, `get`
- [[Embedder]] — from `probe`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
