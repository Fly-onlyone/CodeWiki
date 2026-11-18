---
tags: [core, cw-embed]
---

# OllamaEmbedder

> A model served by Ollama on this machine or another.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `OllamaEmbedder`

## How it works

Declared as `pub struct OllamaEmbedder`.

Public API:

- `fn name(&self) -> String`
- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>`
- `fn probe(&self) -> Result<usize>`

## Depends on

- [[cw-embed]] — uses `BATCH`, `PROBE_TEXT`, `PROBE_TIMEOUT`, `REQUEST_TIMEOUT`
- [[Error Module]] — uses `Result`
- [[Embedder]] — uses `Embedder`

## Used by

- [[cw-embed]] — from `a_probe_embeds_one_word_and_reports_the_vector_length`, `embedders_report_http_errors_and_short_responses`, `from_config`, `ollama_embedder_posts_a_batch_and_normalizes_vectors`

## See also

- [[_index]]
- [[cw-embed]]
