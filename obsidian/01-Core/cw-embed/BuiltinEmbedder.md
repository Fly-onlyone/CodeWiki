---
tags: [core, cw-embed]
---

# BuiltinEmbedder

> A neural model of CodeWiki's catalog, installed under the data directory and run by a hidden child process that CodeWiki starts on the first request and stops when it is idle.

## What it does

An embedder of this kind is only a name and an address: making one starts nothing, so the many places that build an embedder to ask its name cost nothing. All of them reach the one child of this process through `cw_models::worker::global`.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `BuiltinEmbedder`

## How it works

1. `from_config_in` in [[cw-embed]] calls `new`. `new` calls `Error::invalid` in [[Error]], then `find`, then `all` (both in [[Catalog Module]]), then `Store::new` in [[Store]], then `cuda_program` in [[Worker Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct BuiltinEmbedder`.

Members:

- `pub fn new(id: &str, device: &str, data_dir: &Path) -> Result<Self>` — the embedder for the catalog model `id`, installed under `data_dir`, on `device` (one of `DEVICES`).
- `pub fn device(&self) -> cw_models::Device` — the device this embedder asks for. The child may still run on the processor, once the graphics card failed too often.
- `fn name(&self) -> String` — public method, returns `String`; called by `BuiltinEmbedder::embed_query`.
- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>` — public method, returns `Result<Vec<Vec<f32>>>`; called by `BuiltinEmbedder::embed_query`.
- `fn embed_query(&self, text: &str) -> Result<Vec<f32>>` — public method, returns `Result<Vec<f32>>`.
- `fn batch_size(&self) -> usize` — public method, returns `usize`.
- `fn slow(&self) -> bool` — public method, returns `bool`.

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
