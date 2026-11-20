---
tags: [core, cw-embed-engine]
---

# Engine

> `Engine` struct defined in `cw_embed_engine` with 3 methods.

## Source

- `crates/cw-embed-engine/src/lib.rs` — definition of `Engine`

## How it works

1. `worker` in [[cw-embed-engine]] calls `load`. `load` calls `weights` in [[cw-embed-engine]].

## Items

Declared as `pub struct Engine`.

Members:

- `pub fn load(entry: &Entry, dir: &Path, device: &Device) -> Result<Self>` — load the model `entry` from `dir`, the folder an install made.
- `pub fn dimensions(&self) -> usize` — public method, returns `usize`.
- `pub fn embed(&self, kind: Kind, texts: &[String]) -> Result<Vec<Vec<f32>>>` — one unit vector per text, in order.

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
