---
tags: [core, cw-embed-engine]
---

# cw-embed-engine

> Runs CodeWiki's built-in embedding models with candle, in the hidden worker process.

## What it does

The engine of the built-in embedding models.

`Engine` loads one model of the catalog from its folder and embeds texts with it, with `candle`. `worker` is the whole of the hidden child process: it loads the model its arguments name and answers the parent's requests until its input closes (`cw_models::worker` is the protocol).

Only the programs that are that child link this crate. Everything else reaches a model through the child, so that a fault in native model code ends the child alone.

What loading each family of models needs was worked out, and checked against reference vectors, in spike S5 (`spikes/VERDICTS.md`).

## Source

- `crates/cw-embed-engine/Cargo.toml` — package manifest
- `crates/cw-embed-engine/src/lib.rs` — crate root

## How it works

Builds a library.

1. `embed_worker` in [[codewiki]] and `run` in [[Embed Cuda Module]] call `worker`. `worker` calls `init` in [[cw-telemetry]], then `find` in [[Catalog Module]], then `device`, then `Engine::load` in [[Engine]], then `refuse`, then `serve` (both in [[Worker Module]]).
2. `Engine::load` in [[Engine]] calls `weights`.
3. `Engine::embed` in [[Engine]] calls `unit`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Defined at the crate root:

- `pub type Error = Box<dyn std::error::Error + Send + Sync>` — any failure of the engine, as text. It never holds a text that was given to embed: the parent process writes it to its log.
- `pub type Result<T> = std::result::Result<T, Error>` — public type alias; used by `Engine::embed`, `Engine::load`, `device`, `unit` and 2 more.
- [[Engine]] — struct
- `fn weights(dir: &Path, device: &Device, first: &str) -> Result<VarBuilder<'static>>` — the weights of a model, read into memory. candle's memory-mapped loader is an `unsafe fn`; this one is not.
- `fn unit(rows: &Tensor) -> Result<Tensor>` — scale every row to unit length.
- `fn device() -> Result<(Device, cw_models::Device)>` — the device this program was built for: the graphics card when it holds the CUDA engine, the processor otherwise.
- `pub fn worker(id: &str, models_dir: &Path) -> ExitCode` — the whole of the worker process: load the model `id` from `models_dir` and answer requests on standard input until it closes. Standard output carries the protocol and nothing else.

And 1 more private item with no comment.

## Depends on

- [[Worker Module]] — uses `Hello`, `PROTOCOL`, `refuse`, `serve`
- [[Engine]] — uses `Engine`, `dimensions`, `embed`, `load`
- [[cw-telemetry]] — uses `init`
- [[Catalog Module]] — uses `find`
- [[Device]] — uses `Device`

## Used by

- [[Engine]] — from `Engine`, `embed`, `load`
- [[codewiki]] — from `embed_worker`
- [[Embed Cuda Module]] — from `run`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
