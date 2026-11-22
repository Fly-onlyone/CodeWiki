---
tags: [core, cw-embed]
---

# OpenAiEmbedder

> Any endpoint that speaks the OpenAI embeddings API.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `OpenAiEmbedder`

## How it works

1. `from_config` and `local_server` in [[cw-embed]] call `new`. `new` calls `rule` in [[Prefix Module]].

## Items

Declared as `pub struct OpenAiEmbedder`.

Members:

- `pub fn new(server: Server, base_url: &str, api_key: Option<&str>, model: &str) -> Self` — public method, returns `Self`; called by `from_config`, `local_server`.
- `fn timeout(&self) -> Duration` — how long a batch may take: until the first success, long enough for the server to load the model.
- `fn refusal(&self, status: u16, detail: &str) -> Error` — the error for a refused batch. A local server that asks for a token says where to set it.
- `fn check_listed(&self) -> Result<()>` — llama.cpp and mistral.rs embed with the model they hold, whatever model a request names: a model swapped in the server would make vectors under the name of the one chosen here, and two models' vectors would be compared. Until a call succeeds, the server's own list must hold the chosen model. A list that cannot be read is an error too.
- `fn vectors(&self, posted: Posted, count: usize) -> Result<Vec<Vec<f32>>>` — the `count` vectors of an answer, or the error of a refusal.
- `one_by_one` — embed `texts` one per request, each cut to `SHORT_CHARS`, and a text refused alone as too long once more at `TINY_CHARS`. The vectors keep the order of the texts.
- `embed_within` — embed `texts`, each with `prefix` in front.
- `fn name(&self) -> String` — public method, returns `String`; called by `OpenAiEmbedder::embed_query`.
- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>` — public method, returns `Result<Vec<Vec<f32>>>`.
- `fn embed_query(&self, text: &str) -> Result<Vec<f32>>` — public method, returns `Result<Vec<f32>>`.
- `fn probe(&self) -> Result<usize>` — public method, returns `Result<usize>`.

And 2 more private items with no comment.

## Depends on

- [[cw-embed]] — uses `BATCH`, `FIRST_LOAD_TIMEOUT`, `PROBE_TEXT`, `PROBE_TIMEOUT`
- [[Server]] — uses `Server`, `id`, `key_env`, `label`
- [[Error]] — uses `Error`, `external`, `invalid`
- [[Prefix Module]] — uses `Rule`, `rule`
- [[Error Module]] — uses `Result`
- [[List Module (cw-embed)]] — uses `listed_by`
- [[Embedder]] — uses `Embedder`

## Used by

- [[cw-embed]] — from `a_batch_refused_as_too_long_is_sent_again_with_shorter_texts`, `a_batch_refused_twice_is_sent_text_by_text`, `a_failed_probe_carries_the_providers_explanation`, `a_known_model_gets_its_prefixes_and_a_name_that_says_so`

## See also

- [[_index]]
- [[cw-embed]]
