---
tags: [core, cw-embed]
---

# OllamaEmbedder

> A model served by Ollama on this machine or another.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `OllamaEmbedder`

## How it works

What the functions of this type call:

1. `embed` calls `embed_within`. `embed_within` calls `clip`, then `post_json`, then `checked`, then `parse_ollama` (all in [[cw-embed]]).
2. `probe` calls `embed_within`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct OllamaEmbedder`.

Members:

- `fn embed_within(&self, texts: &[String], timeout: Duration) -> Result<Vec<Vec<f32>>>` — private method, returns `Result<Vec<Vec<f32>>>`; called by `OllamaEmbedder::embed`, `OllamaEmbedder::probe`.
- `fn name(&self) -> String` — public method, returns `String`.
- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>` — public method, returns `Result<Vec<Vec<f32>>>`.
- `fn probe(&self) -> Result<usize>` — public method, returns `Result<usize>`.

## Depends on

- [[cw-embed]] — uses `BATCH`, `PROBE_TEXT`, `PROBE_TIMEOUT`, `REQUEST_TIMEOUT`
- [[Error Module]] — uses `Result`
- [[Embedder]] — uses `Embedder`

## Used by

- [[cw-embed]] — from `a_probe_embeds_one_word_and_reports_the_vector_length`, `embedders_report_http_errors_and_short_responses`, `from_config_in`, `ollama_embedder_posts_a_batch_and_normalizes_vectors`

## See also

- [[_index]]
- [[cw-embed]]
