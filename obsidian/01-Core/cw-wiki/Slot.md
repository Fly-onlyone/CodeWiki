---
tags: [core, cw-wiki]
---

# Slot

> The place that holds the embedder in use.

## What it does

Holds the one embedder a process uses for semantic search, behind a lock, so another can be swapped in while the program runs. A single static `Slot` serves the whole process. The backend reconfigures it when the user changes the embedding settings, so no restart is needed.

## Source

- `crates/cw-wiki/src/semantic.rs` — definition of `Slot`

## How it works

The slot starts empty, meaning nothing was chosen. The first `get` takes the write lock and fills it from the environment alone (`from_env`). `configure` builds an embedder from the resolved settings and replaces the slot content. `get` hands out an `Arc` clone, so work that already took the old embedder finishes with it. The stored value has two levels: not chosen yet, or chosen as an embedder or as none. A failed build is logged and stored as none.

## Items

Declared as `pub struct Slot`.

Members:

- `pub const fn new() -> Self` — public method, returns `Self`; called by `EMBEDDER`, `Slot::default`.
- `pub fn get(&self) -> Option<Arc<dyn Embedder>>` — the embedder in use. Before `Slot::configure` is called it is the one the environment alone names, so a program that never reads the settings file (a test, above all) is not changed by one.
- `pub fn configure(&self, resolved: &Resolved)` — use the embedder the settings in effect name, from now on.
- `fn default() -> Self` — public method, returns `Self`.

## Depends on

- [[cw-embed]] — uses `from_env`, `from_settings`
- [[Semantic Module]] — uses `built`, `embedder`
- [[Resolved]] — uses `Resolved`
- [[Embedder]] — uses `Embedder`

## Used by

- [[Semantic Module]] — from `EMBEDDER`, `configure`

## Gotchas

- Before `configure` is called, only the environment counts. A program that never reads the settings file, a test above all, is not changed by one.
- A lock poisoned by a panic is recovered, not propagated.
- A setting that cannot be used turns semantic search off, and a later good one turns it on again.

## See also

- [[_index]]
- [[Semantic Module]]
