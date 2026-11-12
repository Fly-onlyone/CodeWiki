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

- `pub fn embed_one(&self, text: &str) -> Vec<f32>`
- `fn default() -> Self`
- `fn name(&self) -> String`
- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>`

## Depends on

- [[cw-embed]] — uses `STOP_WORDS`, `hash`, `normalize`, `stem`
- [[Error Module]] — uses `Result`
- [[Embedder]] — uses `Embedder`

## Used by

- [[cw-embed]] — from `from_config`

## See also

- [[_index]]
- [[cw-embed]]
