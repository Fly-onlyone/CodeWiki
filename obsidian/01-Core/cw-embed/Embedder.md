---
tags: [core, cw-embed]
---

# Embedder

> `Embedder` trait defined in `cw_embed` with 6 methods.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `Embedder`

## How it works

Declared as `pub trait Embedder: Send + Sync`.

Required and provided methods:

- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>` — one vector per text, in order.
- `fn name(&self) -> String` — identifier stored with each vector.
- `fn embed_query(&self, text: &str) -> Result<Vec<f32>>` — the vector of what a user searches for.
- `fn batch_size(&self) -> usize` — how many documents to hand to one `Embedder::embed` call when many wait.
- `fn slow(&self) -> bool` — whether a repository's documents take it minutes.
- `fn probe(&self) -> Result<usize>` — check that the embedder answers: embed one word and return the length of its vector.

Implemented by [[BuiltinEmbedder]], [[LocalEmbedder]], [[OllamaEmbedder]], [[OpenAiEmbedder]], `StandIn`, and `Other`.

## Depends on

- [[cw-embed]] — uses `BATCH`, `PROBE_TEXT`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `invalid`

## Used by

- [[Semantic Module]] — from `built`, `embed_documents`, `embed_documents_with`, `embed_within`
- [[cw-embed]] — from `from_config`, `from_config_in`, `from_env`, `from_settings`
- [[cw-server]] — from `StandIn`, `stand_in`
- [[Models Module]] — from `embed_repo`, `wait_out_game`
- [[Slot]] — from `Slot`, `get`
- [[cw-wiki]] — from `Other`
- [[Service Module]] — from `reembed`
- [[BuiltinEmbedder]] — from `BuiltinEmbedder`

## See also

- [[_index]]
- [[cw-embed]]
- [[Implementing Embedder Pattern]]
