---
tags: [core, cw-models]
---

# cw-models

> Built-in embedding models of CodeWiki: the catalog, the machine's hardware, the recommendation and the model files on disk.

## What it does

The embedding models CodeWiki can fetch and run by itself.

- `catalog`: the models, a list shipped inside the program. Every file of every model is pinned by revision and SHA-256.
- `hardware`: what the machine has (memory, cores, an NVIDIA card, free disk space).
- `recommend`: which model fits that machine, and why.
- `store`: the models on disk, under `<data dir>\models`: download with resume, verification, removal.
- `worker`: the hidden child process that holds a model, its protocol and the supervisor that starts and stops it.
- `game`: whether a game is running, when the worker and background embedding give way.

Nothing here runs a model. The engine that does is a crate of its own, linked only into the programs that run one, so that this crate stays light for everything else that links it.

## Source

- `crates/cw-models/Cargo.toml` — package manifest
- `crates/cw-models/src/lib.rs` — crate root

## How it works

Builds a library.

## Items

Modules:

- [[Catalog Module]] — the models CodeWiki offers, as a list inside the program.
- [[Game Module]] — whether a game is running.
- [[Hardware Module]] — what the machine has to run a model with.
- [[Recommend Module]] — which model fits a machine.
- [[Store Module (cw-models)]] — the models on disk.
- [[Worker Module]] — the child process that holds a model, seen from both sides.

Defined at the crate root:

- `pub const ENGINE: bool` — whether this build can run a built-in model. The engine is compiled for Windows only.

## Used by

- [[Models Module]] — from `describe`, `unavailable`
- [[Model Module (codewiki)]] — from `run`
- [[Settings Module (cw-server)]] — from `describe`
- [[BuiltinEmbedder]] — from `new`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
