---
tags: [core, cw-embed]
---

# OpenAiEmbedder

> Any endpoint that speaks the OpenAI embeddings API.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `OpenAiEmbedder`

## How it works

What the functions of this type call:

1. `embed` calls `embed_within`. `embed_within` calls `clip`, then `post_json`, then `checked`, then `parse_openai` (all in [[cw-embed]]).
2. `probe` calls `embed_within`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct OpenAiEmbedder`.

Members:

- `fn embed_within(&self, texts: &[String], timeout: Duration) -> Result<Vec<Vec<f32>>>` — private method, returns `Result<Vec<Vec<f32>>>`; called by `OpenAiEmbedder::embed`, `OpenAiEmbedder::probe`.
- `fn name(&self) -> String` — public method, returns `String`.
- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>` — public method, returns `Result<Vec<Vec<f32>>>`.
- `fn probe(&self) -> Result<usize>` — public method, returns `Result<usize>`.

## Depends on

- [[cw-embed]] — uses `BATCH`, `PROBE_TEXT`, `PROBE_TIMEOUT`, `REQUEST_TIMEOUT`
- [[Error Module]] — uses `Result`
- [[Embedder]] — uses `Embedder`

## Used by

- [[cw-embed]] — from `a_failed_probe_carries_the_providers_explanation`, `a_probe_embeds_one_word_and_reports_the_vector_length`, `from_config_in`, `openai_embedder_sends_a_bearer_token_and_orders_by_index`

## See also

- [[_index]]
- [[cw-embed]]
