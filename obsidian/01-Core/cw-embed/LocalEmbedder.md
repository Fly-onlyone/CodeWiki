---
tags: [core, cw-embed]
---

# LocalEmbedder

> The built-in embedder: signed feature hashing, no model and no network.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `LocalEmbedder`

## How it works

1. `from_config_in` in [[cw-embed]] calls `default`.

## Items

Declared as `pub struct LocalEmbedder`.

Members:

- `fn default() -> Self` — public method, returns `Self`; called by `from_config_in`.
- `pub fn embed_one(&self, text: &str) -> Vec<f32>` — public method, returns `Vec<f32>`; called by `LocalEmbedder::embed`.
- `fn name(&self) -> String` — public method, returns `String`.
- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>` — public method, returns `Result<Vec<Vec<f32>>>`.

And 1 more private item with no comment.

## Depends on

- [[cw-embed]] — uses `STOP_WORDS`, `hash`, `normalize`, `stem`
- [[Error Module]] — uses `Result`
- [[Embedder]] — uses `Embedder`

## Used by

- [[cw-embed]] — from `a_probe_embeds_one_word_and_reports_the_vector_length`, `from_config_in`
- [[cw-server]] — from `StandIn`, `stand_in`
- [[cw-wiki]] — from `Other`, `the_documents_of_an_index_are_embedded_again_in_steps`

## See also

- [[_index]]
- [[cw-embed]]
