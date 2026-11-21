---
tags: [core, cw-embed]
---

# LocalEmbedder

> The built-in embedder: signed feature hashing, no model and no network.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `LocalEmbedder`

## How it works

1. `from_config` in [[cw-embed]] calls `default`.

## Items

Declared as `pub struct LocalEmbedder`.

Members:

- `fn default() -> Self` — public method, returns `Self`; called by `from_config`.
- `pub fn embed_one(&self, text: &str) -> Vec<f32>` — public method, returns `Vec<f32>`; called by `LocalEmbedder::embed`.
- `fn name(&self) -> String` — public method, returns `String`.
- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>` — public method, returns `Result<Vec<Vec<f32>>>`.

And 1 more private item with no comment.

## Depends on

- [[cw-embed]] — uses `STOP_WORDS`, `hash`, `normalize`, `stem`
- [[Error Module]] — uses `Result`
- [[Embedder]] — uses `Embedder`

## Used by

- [[cw-server]] — from `StandIn`, `held`, `stand_in`
- [[cw-embed]] — from `a_probe_embeds_one_word_and_reports_the_vector_length`, `from_config`

## See also

- [[_index]]
- [[cw-embed]]
