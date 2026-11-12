---
tags: [core, cw-embed]
---

# Embedder

> `Embedder` trait defined in `cw_embed` with 2 methods.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `Embedder`

## How it works

Declared as `pub trait Embedder: Send + Sync`.

Required and provided methods:

- `fn name(&self) -> String` — identifier stored with each vector.
- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>` — one vector per text, in order.

Implemented by [[LocalEmbedder]], `OllamaEmbedder`, and `OpenAiEmbedder`.

## Depends on

- [[Error Module]] — uses `Result`

## Used by

- [[cw-embed]] — from `OllamaEmbedder`, `OpenAiEmbedder`, `from_config`, `from_env`
- [[Semantic Module]] — from `EMBEDDER`, `embed_documents`, `embedder`, `search_with`
- [[LocalEmbedder]] — from `LocalEmbedder`

## See also

- [[_index]]
- [[cw-embed]]
- [[Implementing Embedder Pattern]]
