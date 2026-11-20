---
tags: [core, cw-embed]
---

# BuiltinEmbedder

> A neural model of CodeWiki's catalog, installed under the data directory and run by a hidden child process that CodeWiki starts on the first request and stops when it is idle.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `BuiltinEmbedder`

## How it works

Declared as `pub struct BuiltinEmbedder`.

Public API:

- `pub fn new(id: &str, device: &str, data_dir: &Path) -> Result<Self>` — the embedder for the catalog model `id`, installed under `data_dir`, on `device` (one of `DEVICES`).
- `pub fn device(&self) -> cw_models::Device` — the device this embedder asks for.
- `fn name(&self) -> String`
- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>`
- `fn embed_query(&self, text: &str) -> Result<Vec<f32>>`
- `fn batch_size(&self) -> usize`

## Depends on

- [[Worker Module]] — uses `Spec`, `cuda_program`, `global`
- [[Store]] — uses `is_installed`, `new`, `root`
- [[cw-embed]] — uses `CUDA_VRAM`, `DEVICES`
- [[Catalog Module]] — uses `all`, `find`
- [[Entry]] — uses `Entry`, `embedder_name`
- [[cw-models]] — uses `ENGINE`
- [[Error Module]] — uses `Result`
- [[Hardware Module]] — uses `nvidia_card`

## Used by

- [[cw-embed]] — from `from_config_in`

## See also

- [[_index]]
- [[cw-embed]]
