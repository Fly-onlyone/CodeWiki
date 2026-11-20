---
tags: [core, cw-wiki]
---

# Slot

> The place that holds the embedder in use.

## Source

- `crates/cw-wiki/src/semantic.rs` — definition of `Slot`

## How it works

Declared as `pub struct Slot`.

It can be given another one while the program runs; work that already took the old one finishes with it.

Public API:

- `pub fn configure_in(&self, resolved: &Resolved, data_dir: &Path)` — `Slot::configure` with the data directory that holds the built-in models.
- `pub fn configure(&self, resolved: &Resolved)` — use the embedder the settings in effect name, from now on.
- `pub fn get(&self) -> Option<Arc<dyn Embedder>>` — the embedder in use.
- `pub const fn new() -> Self`
- `fn default() -> Self`

## Depends on

- [[cw-embed]] — uses `from_env`, `from_settings_in`
- [[Semantic Module]] — uses `built`, `embedder`
- [[Paths Module]] — uses `data_dir`
- [[Resolved]] — uses `Resolved`
- [[Embedder]] — uses `Embedder`

## Used by

- [[Semantic Module]] — from `EMBEDDER`, `configure`, `configure_in`

## See also

- [[_index]]
- [[Semantic Module]]
