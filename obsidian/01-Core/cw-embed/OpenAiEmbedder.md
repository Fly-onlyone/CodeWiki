---
tags: [core, cw-embed]
---

# OpenAiEmbedder

> Any endpoint that speaks the OpenAI embeddings API.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `OpenAiEmbedder`

## How it works

Declared as `pub struct OpenAiEmbedder`.

Public API:

- `fn name(&self) -> String`
- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>`
- `fn probe(&self) -> Result<usize>`

## Depends on

- [[cw-embed]] — uses `BATCH`, `PROBE_TEXT`, `PROBE_TIMEOUT`, `REQUEST_TIMEOUT`
- [[Error Module]] — uses `Result`
- [[Embedder]] — uses `Embedder`

## Used by

- [[cw-embed]] — from `a_failed_probe_carries_the_providers_explanation`, `a_probe_embeds_one_word_and_reports_the_vector_length`, `from_config_in`, `openai_embedder_sends_a_bearer_token_and_orders_by_index`

## See also

- [[_index]]
- [[cw-embed]]
