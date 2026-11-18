---
tags: [core, cw-embed]
---

# LocalEmbedder

> The built-in embedder: signed feature hashing, no model and no network.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `LocalEmbedder`

## How it works

Declared as `pub struct LocalEmbedder`.

Public API:

- `fn default() -> Self`
- `pub fn embed_one(&self, text: &str) -> Vec<f32>`
- `fn name(&self) -> String`
- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>`

## Depends on

- [[cw-embed]] — uses `STOP_WORDS`, `hash`, `normalize`, `stem`
- [[Error Module]] — uses `Result`
- [[Embedder]] — uses `Embedder`

## Used by

- [[cw-embed]] — from `a_probe_embeds_one_word_and_reports_the_vector_length`, `from_config`

## See also

- [[_index]]
- [[cw-embed]]
