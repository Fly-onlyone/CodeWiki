---
tags: [core, cw-embed-engine]
---

# cw-embed-engine

> Runs CodeWiki's built-in embedding models with candle, in the hidden worker process.

## Source

- `crates/cw-embed-engine/Cargo.toml` — package manifest
- `crates/cw-embed-engine/src/lib.rs` — crate root

## How it works

Builds a library.

Defined at the crate root:

- `Error` — any failure of the engine, as text.
- [[Engine]] — struct
- `Result` — type alias
- `worker` — the whole of the worker process: load the model `id` from `models_dir` and answer requests on standard input until it closes.
- `device` — the device this program was built for: the graphics card when it holds the CUDA engine, the processor otherwise.
- `Model` — enum
- `weights` — the weights of a model, read into memory.
- `unit` — scale every row to unit length.

## Depends on

- [[Worker Module]] — uses `Hello`, `PROTOCOL`, `refuse`, `serve`
- [[Engine]] — uses `Engine`, `dimensions`, `embed`, `load`
- [[Logs Module]] — uses `init`
- [[Catalog Module]] — uses `find`
- [[Device]] — uses `Device`

## Used by

- [[Engine]] — from `Engine`, `embed`, `load`
- [[codewiki]] — from `embed_worker`
- [[Embed Cuda Module]] — from `run`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
