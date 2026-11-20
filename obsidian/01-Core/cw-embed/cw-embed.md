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
- [[BuiltinEmbedder]] — A neural model of CodeWiki's catalog, installed under the data directory and run by a hidden child process that CodeWiki starts on the first request and stops when it is idle.
- [[OpenAiEmbedder]] — any endpoint that speaks the OpenAI embeddings API.
- [[OllamaEmbedder]] — A model served by Ollama on this machine or another.
- `normalize` — scale `vector` to unit length.
- `from_config_in` — `from_config` with the data directory that holds the built-in models.
- `from_settings_in` — `from_settings` with the data directory that holds the built-in models.

## Depends on

- [[Error]] — uses `external`, `invalid`
- [[LocalEmbedder]] — uses `default`, `embed_one`
- [[Error Module]] — uses `Result`
- [[Logs Module]] — uses `flush`
- [[Paths Module]] — uses `data_dir`
- [[Resolved]] — uses `Resolved`
- [[BuiltinEmbedder]] — uses `new`
- [[Embedder]] — uses `Embedder`

## Used by

- [[Settings Module (cw-server)]] — from `change`, `checked`, `describe`, `test_embedder`
- [[OllamaEmbedder]] — from `embed`, `embed_within`, `probe`
- [[OpenAiEmbedder]] — from `embed`, `embed_within`, `probe`
- [[Embedder]] — from `batch_size`, `probe`
- [[LocalEmbedder]] — from `add`, `embed_one`
- [[Slot]] — from `configure_in`, `get`
- [[BuiltinEmbedder]] — from `new`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
