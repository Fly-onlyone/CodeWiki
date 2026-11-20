---
tags: [core, cw-embed]
---

# cw-embed

> Text embeddings for CodeWiki's semantic search: a built-in local embedder and HTTP providers.

## What it does

Text embeddings for semantic search.

Four embedders sit behind one trait:

- `LocalEmbedder` runs in-process with no model file and no network. It hashes words, word pairs and character trigrams into a fixed-size vector, so texts that share vocabulary, word stems or identifier parts land close together. It does not know that two unrelated words mean the same thing; a neural model does.
- `BuiltinEmbedder` has a neural model of CodeWiki's own catalog embed, in a hidden child process that CodeWiki starts by itself.
- `OllamaEmbedder` calls a model served by a local Ollama.
- `OpenAiEmbedder` calls any OpenAI-compatible embeddings endpoint.

`CODEWIKI_EMBED` selects one: `local` (default), `builtin`, `ollama`, `openai` or `none`.

## Source

- `crates/cw-embed/Cargo.toml` — package manifest
- `crates/cw-embed/src/lib.rs` — crate root

## How it works

Builds a library.

1. `OllamaEmbedder::embed_within` in [[OllamaEmbedder]] and `OpenAiEmbedder::embed_within` in [[OpenAiEmbedder]] call `clip`.
2. `OllamaEmbedder::embed_within` in [[OllamaEmbedder]] and `OpenAiEmbedder::embed_within` in [[OpenAiEmbedder]] call `post_json`. `post_json` calls `Error::external` in [[Error]].
3. `OllamaEmbedder::embed_within` in [[OllamaEmbedder]] and `OpenAiEmbedder::embed_within` in [[OpenAiEmbedder]] call `checked`. `checked` calls `Error::invalid` in [[Error]].
4. `change` and `describe` in [[Settings Module (cw-server)]], and 2 more, call `from_settings_in`. `from_settings_in` calls `from_config_in`. `from_config_in` calls `LocalEmbedder::default` in [[LocalEmbedder]], then `Error::invalid` in [[Error]], then `BuiltinEmbedder::new` in [[BuiltinEmbedder]].
5. `LocalEmbedder::embed_one` in [[LocalEmbedder]] calls `normalize`.

6 more entry points: `words`, `stem`, `hash`, `parse_ollama`, `parse_openai`, `from_env`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Defined at the crate root:

- `pub const EMBED_ENV: &str` — environment variable selecting the embedder.
- `pub const EMBED_MODEL_ENV: &str` — model name for the HTTP embedders.
- `pub const EMBED_DEVICE_ENV: &str` — what runs a built-in model.
- `pub const PROVIDERS: [&str; 5]` — the embedders a user can choose, in the order a page lists them.
- `pub const DEVICES: [&str; 3]` — the values of `EMBED_DEVICE_ENV`.
- [[Embedder]] — trait
- `pub fn normalize(vector: &mut [f32])` — scale `vector` to unit length. A zero vector stays zero.
- `pub fn similarity(a: &[f32], b: &[f32]) -> f32` — cosine similarity of two unit vectors; 0 when their lengths differ.
- `pub fn words(text: &str) -> Vec<String>` — split text into lowercase words, breaking identifiers at underscores, punctuation and camel-case humps: `PageRank_score` gives `page`, `rank`, `score`.
- `pub fn stem(word: &str) -> &str` — strip a common English ending so `computes`, `computed` and `computing` count as one word. Crude on purpose: it only has to be consistent.
- `fn hash(feature: &str, salt: u8) -> u64` — FNV-1a, 64 bit. Stable across runs and platforms, which the stored vectors depend on.
- [[LocalEmbedder]] — the built-in embedder: signed feature hashing, no model and no network.
- [[OllamaEmbedder]] — A model served by Ollama on this machine or another.
- [[OpenAiEmbedder]] — any endpoint that speaks the OpenAI embeddings API.
- [[BuiltinEmbedder]] — A neural model of CodeWiki's catalog, installed under the data directory and run by a hidden child process that CodeWiki starts on the first request and stops when it is idle.
- `pub fn from_config(lookup: impl Fn(&str) -> Option<String>) -> Result<Option<Box<dyn Embedder>>>` — the embedder for a provider name. `lookup` reads configuration values; production code passes the process environment. A built-in model is looked for under the default data directory.
- `from_config_in` — `from_config` with the data directory that holds the built-in models.
- `pub fn from_env() -> Result<Option<Box<dyn Embedder>>>` — the embedder chosen by the environment; `None` turns semantic search off.
- `pub fn from_settings(resolved: &cw_core::settings::Resolved) -> Result<Option<Box<dyn Embedder>>>` — the embedder chosen by the settings in effect: the environment over the settings file.
- `from_settings_in` — `from_settings` with the data directory that holds the built-in models. A backend passes its own, which need not be the default one.

And 14 more items.

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
