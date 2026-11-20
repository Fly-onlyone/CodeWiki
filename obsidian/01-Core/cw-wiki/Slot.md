---
tags: [core, cw-wiki]
---

# Slot

> The place that holds the embedder in use.

## What it does

It can be given another one while the program runs; work that already took the old one finishes with it.

## Source

- `crates/cw-wiki/src/semantic.rs` — definition of `Slot`

## How it works

1. `EMBEDDER` in [[Semantic Module]] calls `new`.

## Items

Declared as `pub struct Slot`.

Members:

- `pub const fn new() -> Self` — public method, returns `Self`; called by `EMBEDDER`, `Slot::default`.
- `pub fn get(&self) -> Option<Arc<dyn Embedder>>` — the embedder in use. Before `Slot::configure` is called it is the one the environment alone names, so a program that never reads the settings file (a test, above all) is not changed by one.
- `pub fn configure(&self, resolved: &Resolved)` — use the embedder the settings in effect name, from now on. A built-in model is looked for under the default data directory.
- `pub fn configure_in(&self, resolved: &Resolved, data_dir: &Path)` — `Slot::configure` with the data directory that holds the built-in models.
- `fn default() -> Self` — public method, returns `Self`.

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
