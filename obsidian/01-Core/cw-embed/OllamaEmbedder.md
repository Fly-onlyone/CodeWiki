---
tags: [core, cw-embed]
---

# OllamaEmbedder

> A model served by Ollama on this machine or another.

## What it does

Turns texts into vectors by asking an Ollama server. Ollama is a local model runner; it can run on this machine or on another one. Semantic search uses these vectors to find text by meaning. It is one of the choices behind the `Embedder` trait, next to the in-process local embedder and the OpenAI-style one.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `OllamaEmbedder`

## How it works

`from_config` builds it when `CODEWIKI_EMBED` is `ollama`. The address comes from `OLLAMA_HOST`, or the standard local Ollama address on port 11434. The model comes from `CODEWIKI_EMBED_MODEL`, or `embeddinggemma`. `new` looks up a prefix rule for the model name with `rule`. Some models want a fixed text before a query and another before a document.

`embed` uses the document prefix. `embed_query` uses the query prefix. `probe` embeds the word `ping` and returns the vector length, with a 60 second limit. All three call `embed_within`.

`embed_within` posts to the `/api/embed` path. It splits the texts into batches of 64. It puts the prefix in front of each text, then clips each text to 8,000 bytes. The model name goes in the body. `parse_ollama` reads the `embeddings` array and scales each vector to unit length. `checked` then confirms the count and that all vectors have the same length.

An HTTP error status becomes an error that carries the server's own message. `name` gives the model name after an `ollama` label, plus the rule id when a prefix rule applies. The stored name decides which vectors are comparable.

## Items

Declared as `pub struct OllamaEmbedder`.

Members:

- `pub fn new(base_url: &str, model: &str) -> Self` — public method, returns `Self`; called by `from_config`.
- `fn timeout(&self) -> Duration` — how long a batch may take: until the first success, long enough for the server to load the model.
- `embed_within` — embed `texts`, each with `prefix` in front.
- `fn name(&self) -> String` — public method, returns `String`; called by `OllamaEmbedder::embed_query`.
- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>` — public method, returns `Result<Vec<Vec<f32>>>`.
- `fn embed_query(&self, text: &str) -> Result<Vec<f32>>` — public method, returns `Result<Vec<f32>>`.
- `fn probe(&self) -> Result<usize>` — public method, returns `Result<usize>`.

## Depends on

- [[cw-embed]] — uses `BATCH`, `FIRST_LOAD_TIMEOUT`, `PROBE_TEXT`, `PROBE_TIMEOUT`
- [[Prefix Module]] — uses `Rule`, `rule`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `invalid`
- [[Embedder]] — uses `Embedder`

## Used by

- [[cw-embed]] — from `a_known_model_gets_its_prefixes_and_a_name_that_says_so`, `a_probe_embeds_one_word_and_reports_the_vector_length`, `embedders_report_http_errors_and_short_responses`, `from_config`

## Gotchas

- The first call gets 300 seconds, because Ollama loads the model on the first request. After one batch succeeds, the `warm` flag is set and later calls get 120 seconds. `probe` always gets 60 seconds, even when cold.
- Unlike the OpenAI-style embedder, it does not retry with shorter texts when the server says the input is too long. A text over the model's limit fails the batch.
- The prefix is part of the stored name. A model that gains a prefix rule gets a new name, so its vectors are embedded again.
- The clip counts bytes, and it cuts on a character boundary.

## See also

- [[_index]]
- [[cw-embed]]
