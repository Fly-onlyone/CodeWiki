---
tags: [core, cw-embed-engine]
---

# Engine

> `Engine` struct defined in `cw_embed_engine` with 3 methods.

## Source

- `crates/cw-embed-engine/src/lib.rs` — definition of `Engine`

## How it works

Declared as `pub struct Engine`.

Public API:

- `pub fn load(entry: &Entry, dir: &Path, device: &Device) -> Result<Self>` — load the model `entry` from `dir`, the folder an install made.
- `pub fn embed(&self, kind: Kind, texts: &[String]) -> Result<Vec<Vec<f32>>>` — one unit vector per text, in order.
- `pub fn dimensions(&self) -> usize`

## Depends on

- [[cw-embed-engine]] — uses `Model`, `Result`, `device`, `unit`
- [[Catalog Module]] — uses `Pooling`
- [[Worker Module]] — uses `Kind`
- [[Entry]] — uses `Entry`

## Used by

- [[cw-embed-engine]] — from `worker`

## See also

- [[_index]]
- [[cw-embed-engine]]
