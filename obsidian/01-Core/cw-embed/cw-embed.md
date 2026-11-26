---
tags: [core, cw-embed]
---

# cw-embed

> Text embeddings for CodeWiki's semantic search: a word-form hashing embedder and HTTP providers.

## What it does

`cw-embed` turns text into vectors for semantic search. It offers one trait, [[Embedder]], and three kinds of implementation: [[LocalEmbedder]] (built in, no network, the default), [[OllamaEmbedder]], and [[OpenAiEmbedder]] for OpenAI, LM Studio, llama.cpp, Lemonade and mistral.rs. It also lists the models a server offers, so the Settings page can show a pick list. Callers use `from_settings` or `from_env` to get the embedder the user chose.

## Source

- `crates/cw-embed/Cargo.toml` — package manifest
- `crates/cw-embed/src/lib.rs` — crate root

## How it works

`from_config` takes a lookup function for settings. It reads `CODEWIKI_EMBED`: no value or `local` gives the local embedder, `none` gives no embedder (semantic search off), `ollama` and `openai` build their own embedders, and the four local servers go through `local_server`, which requires `CODEWIKI_EMBED_MODEL`. `from_env` uses the process environment; `from_settings` uses the settings in effect, where the environment wins over the settings file.

The HTTP embedders share helpers. `post_json` and `get_json` make the call with a timeout and return `Posted`: either the JSON or the refusal with the server's explanation. `prefixed` puts the model's prefix in front, `clip` and `cut` limit length on a character boundary, and `checked` verifies the vector count and equal lengths. `normalize` scales every vector to unit length, so `similarity` is just a dot product.

## Items

Modules:

- [[List Module (cw-embed)]] — the models a server offers, for the page that lets someone pick one.
- [[Prefix Module]] — the text some embedding models expect before a query and before a document.

Defined at the crate root:

- `pub const EMBED_ENV: &str` — environment variable selecting the embedder.
- `pub const EMBED_MODEL_ENV: &str` — model name for the HTTP embedders.
- `pub const PROVIDERS: [&str; 8]` — the embedders a user can choose, in the order a page lists them.
- [[Embedder]] — trait
- `pub fn normalize(vector: &mut [f32])` — scale `vector` to unit length. A zero vector stays zero.
- `pub fn similarity(a: &[f32], b: &[f32]) -> f32` — cosine similarity of two unit vectors; 0 when their lengths differ.
- `pub fn words(text: &str) -> Vec<String>` — split text into lowercase words, breaking identifiers at underscores, punctuation and camel-case humps: `PageRank_score` gives `page`, `rank`, `score`.
- `pub fn stem(word: &str) -> &str` — strip a common English ending so `computes`, `computed` and `computing` count as one word. Crude on purpose: it only has to be consistent.
- `fn hash(feature: &str, salt: u8) -> u64` — FNV-1a, 64 bit. Stable across runs and platforms, which the stored vectors depend on.
- [[LocalEmbedder]] — the word-form embedder: signed feature hashing, no model and no network.
- `fn named(base: String, rule: Option<&prefix::Rule>) -> String` — A vector's name: the model, plus `+id` of the prefix rule that applies.
- `fn cut(text: &str, max: usize) -> &str` — `text` cut to at most `max` bytes, on a character boundary.
- [[OllamaEmbedder]] — A model served by Ollama on this machine or another.
- [[Server]] — A server that speaks the OpenAI embeddings API.
- `pub fn server_root(base: &str) -> &str` — an address without its trailing `/` and one trailing `/v1`. LM Studio, Lemonade and mistral.rs show their address with `/v1`; both forms work.
- [[OpenAiEmbedder]] — any endpoint that speaks the OpenAI embeddings API.
- `pub fn from_config(lookup: impl Fn(&str) -> Option<String>) -> Result<Option<Box<dyn Embedder>>>` — the embedder for a provider name. `lookup` reads configuration values; production code passes the process environment.
- `pub fn list_models_in(resolved: &cw_core::settings::Resolved) -> Result<Option<Vec<Listed>>>` — the models the provider chosen by the settings in effect offers.
- `pub fn from_env() -> Result<Option<Box<dyn Embedder>>>` — the embedder chosen by the environment; `None` turns semantic search off.
- `pub fn from_settings(resolved: &cw_core::settings::Resolved) -> Result<Option<Box<dyn Embedder>>>` — the embedder chosen by the settings in effect: the environment over the settings file.

And 26 more items.

## Depends on

- [[Server]] — uses `Server`, `address`, `default_url`, `id`
- [[Error]] — uses `Error`, `external`, `invalid`
- [[OpenAiEmbedder]] — uses `OpenAiEmbedder`, `new`, `post`
- [[List Module (cw-embed)]] — uses `Listed`, `list_models`
- [[LocalEmbedder]] — uses `default`, `embed_one`
- [[Error Module]] — uses `Result`
- [[Logs Module]] — uses `flush`
- [[Prefix Module]] — uses `Rule`

## Used by

- [[OpenAiEmbedder]] — from `embed_within`, `name`, `one_by_one`, `post`
- [[List Module (cw-embed)]] — from `fetch`, `list_models`, `list_ollama`, `listed_by`
- [[Settings Module (cw-server)]] — from `checked`, `describe`, `embed_models`, `test_embedder`
- [[OllamaEmbedder]] — from `embed_within`, `name`, `probe`, `timeout`
- [[Embedder]] — from `batch_size`, `probe`
- [[LocalEmbedder]] — from `add`, `embed_one`
- [[Slot]] — from `configure`, `get`

## Gotchas

- The choice `builtin` is refused with a message that the built-in model was removed. An unknown name lists the valid ones.
- Vectors from different embedders are never compared. The embedder's name is stored with each vector, so switching provider or model re-embeds everything.
- A server error on a refused batch carries the server's own text, not a guess.
- `local_server` has no default model; without one it tells the user to choose in Settings, Semantic search.
- `OPENAI_API_KEY` is required for `openai`. A missing key is an error at build time, not at the first call.

## See also

- [[_index]]
- [[Crate Dependency Flow]]
