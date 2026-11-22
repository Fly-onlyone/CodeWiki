---
tags: [core, cw-embed]
---

# Embedder

> `Embedder` trait defined in `cw_embed` with 5 methods.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `Embedder`

## How it works

What the functions of this type call:

1. `embed_query` calls `embed`, then `Error::invalid` in [[Error]], then `name`.
2. `probe` calls `embed`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub trait Embedder: Send + Sync`.

Required and provided methods:

- `fn name(&self) -> String` — identifier stored with each vector. Vectors from different embedders are never compared, so a change of name re-embeds everything.
- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>` — one vector per text, in order. Every vector has the same length.
- `fn embed_query(&self, text: &str) -> Result<Vec<f32>>` — the vector of what a user searches for. Some models put an instruction before a query and none before a document; the others embed both alike, which is the default.
- `fn batch_size(&self) -> usize` — how many documents to hand to one `Embedder::embed` call when many wait. Between two calls a long run reports how far it is and can be stopped, so a slow embedder names a small number.
- `fn probe(&self) -> Result<usize>` — check that the embedder answers: embed one word and return the length of its vector. An error carries the provider's own explanation.

Implemented by [[LocalEmbedder]], [[OllamaEmbedder]], [[OpenAiEmbedder]], `Failing`, and `StandIn`.

## Depends on

- [[cw-embed]] — uses `BATCH`, `PROBE_TEXT`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `invalid`

## Used by

- [[cw-server]] — from `Failing`, `StandIn`, `config`, `down_then_up`
- [[Semantic Module]] — from `built`, `embed_documents`, `embed_documents_with`, `embed_within`
- [[cw-embed]] — from `from_config`, `from_env`, `from_settings`
- [[Embed Job Module]] — from `embed_all`, `embed_repo`, `run`
- [[Slot]] — from `Slot`, `get`
- [[LocalEmbedder]] — from `LocalEmbedder`
- [[OllamaEmbedder]] — from `OllamaEmbedder`
- [[OpenAiEmbedder]] — from `OpenAiEmbedder`

## See also

- [[_index]]
- [[cw-embed]]
- [[Implementing Embedder Pattern]]
