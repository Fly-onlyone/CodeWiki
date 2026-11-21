---
tags: [core, cw-embed]
---

# cw-embed

> Text embeddings for CodeWiki's semantic search: a built-in local embedder and HTTP providers.

## What it does

Text embeddings for semantic search.

These embedders sit behind one trait:

- `LocalEmbedder` runs in-process with no model file and no network. It hashes words, word pairs and character trigrams into a fixed-size vector, so texts that share vocabulary, word stems or identifier parts land close together. It does not know that two unrelated words mean the same thing; a neural model does.
- `OllamaEmbedder` calls a model served by a local Ollama.
- `OpenAiEmbedder` calls any OpenAI-compatible embeddings endpoint. It also serves LM Studio, llama.cpp, Lemonade and mistral.rs, which take the same request at `/v1/embeddings` (see `Server`).

`CODEWIKI_EMBED` selects one: `local` (default), `lmstudio`, `ollama`, `llamacpp`, `lemonade`, `mistralrs`, `openai` or `none`.

## Source

- `crates/cw-embed/Cargo.toml` — package manifest
- `crates/cw-embed/src/lib.rs` — crate root

## How it works

Builds a library.

1. `list_ollama` in [[List Module (cw-embed)]] and `OllamaEmbedder::embed_within` in [[OllamaEmbedder]], and 1 more, call `post_json`. `post_json` calls `agent`, then `Error::external` in [[Error]], then `finish`. `finish` calls `Error::external` in [[Error]].
2. `fetch` in [[List Module (cw-embed)]] and `OllamaEmbedder::embed_within` in [[OllamaEmbedder]], and 1 more, call `refused`. `refused` calls `Error::external` in [[Error]].
3. `OllamaEmbedder::embed_within` in [[OllamaEmbedder]] and `OpenAiEmbedder::embed_within` in [[OpenAiEmbedder]] call `prefixed`.
4. `OllamaEmbedder::name` in [[OllamaEmbedder]] and `OpenAiEmbedder::name` in [[OpenAiEmbedder]] call `named`.
5. `OllamaEmbedder::embed_within` in [[OllamaEmbedder]] and `OpenAiEmbedder::embed_within` in [[OpenAiEmbedder]] call `clip`. `clip` calls `cut`.

14 more entry points: `checked`, `server_root`, `from_settings`, `normalize`, `words`, `stem`, and 8 more.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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
- [[LocalEmbedder]] — the built-in embedder: signed feature hashing, no model and no network.
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

And 24 more items.

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

- [[OpenAiEmbedder]] — from `embed_within`, `name`, `post`, `probe`
- [[Settings Module (cw-server)]] — from `checked`, `describe`, `embed_models`, `test_embedder`
- [[OllamaEmbedder]] — from `embed_within`, `name`, `probe`, `timeout`
- [[List Module (cw-embed)]] — from `fetch`, `list_models`, `list_ollama`
- [[Embedder]] — from `batch_size`, `probe`
- [[LocalEmbedder]] — from `add`, `embed_one`
- [[Slot]] — from `configure`, `get`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
