---
tags: [core, cw-embed]
---

# Embedder

> `Embedder` trait defined in `cw_embed` with 5 methods.

## What it does

`Embedder` is the one interface that turns text into number vectors for semantic search. The rest of CodeWiki holds a `dyn Embedder` and does not care whether the vectors come from the built-in hashing code, Ollama, or an OpenAI-style server. Two texts with close vectors mean similar things, so a search question can find notes and symbols by meaning.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `Embedder`

## How it works

An implementor gives `name` and `embed`. The other methods have defaults. `embed_query` embeds one text and takes the first vector; it returns an error if none comes back. A model that wants a different prefix for queries overrides it ([[OllamaEmbedder]] and [[OpenAiEmbedder]] do). `batch_size` returns the crate's batch constant of 64; callers such as the embed job loop over documents in chunks of that size so they can report progress and stop between calls. `probe` embeds the word "ping" and returns the vector length, which the Settings page uses for its Test button.

The implementors are [[LocalEmbedder]], [[OllamaEmbedder]] and [[OpenAiEmbedder]], plus test doubles.

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

- [[cw-server]] — from `Failing`, `Host`, `StandIn`, `config`
- [[Semantic Module]] — from `built`, `embed_documents`, `embed_documents_with`, `embed_within`
- [[cw-embed]] — from `from_config`, `from_env`, `from_settings`
- [[Embed Job Module]] — from `embed_all`, `embed_repo`, `run`
- [[Slot]] — from `Slot`, `get`
- [[LocalEmbedder]] — from `LocalEmbedder`
- [[OllamaEmbedder]] — from `OllamaEmbedder`
- [[OpenAiEmbedder]] — from `OpenAiEmbedder`

## Gotchas

- `name` is stored with each vector. Vectors from different names are never compared, so a changed name, including a model change or a prefix rule, makes everything embed again.
- The trait is `Send + Sync`, so an implementor must be safe to share between threads. The HTTP embedders keep their warm-up flag in an atomic for this reason.
- Every vector from one `embed` call must have the same length, and `embed` must return one per text, in order.

## See also

- [[_index]]
- [[cw-embed]]
- [[Implementing Embedder Pattern]]
