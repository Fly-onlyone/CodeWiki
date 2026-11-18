---
tags: [core, cw-embed]
---

# Embedder

> `Embedder` trait defined in `cw_embed` with 3 methods.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `Embedder`

## How it works

Declared as `pub trait Embedder: Send + Sync`.

Required and provided methods:

- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>` — one vector per text, in order.
- `fn name(&self) -> String` — identifier stored with each vector.
- `fn probe(&self) -> Result<usize>` — check that the embedder answers: embed one word and return the length of its vector.

Implemented by [[LocalEmbedder]], [[OllamaEmbedder]], and [[OpenAiEmbedder]].

## Depends on

- [[cw-embed]] — uses `PROBE_TEXT`
- [[Error Module]] — uses `Result`

## Used by

- [[Semantic Module]] — from `built`, `embed_documents`, `embedder`, `nearest`
- [[cw-embed]] — from `from_config`, `from_env`, `from_settings`
- [[Slot]] — from `Slot`, `get`
- [[LocalEmbedder]] — from `LocalEmbedder`
- [[OllamaEmbedder]] — from `OllamaEmbedder`
- [[OpenAiEmbedder]] — from `OpenAiEmbedder`

## See also

- [[_index]]
- [[cw-embed]]
- [[Implementing Embedder Pattern]]
