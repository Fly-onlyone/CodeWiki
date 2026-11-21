---
tags: [core, cw-embed]
---

# OllamaEmbedder

> A model served by Ollama on this machine or another.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `OllamaEmbedder`

## How it works

1. `from_config` in [[cw-embed]] calls `new`. `new` calls `rule` in [[Prefix Module]].

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

## See also

- [[_index]]
- [[cw-embed]]
