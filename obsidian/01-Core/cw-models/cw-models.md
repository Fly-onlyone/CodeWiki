---
tags: [core, cw-models]
---

# cw-models

> Built-in embedding models of CodeWiki: the catalog, the machine's hardware, the recommendation and the model files on disk.

## Source

- `crates/cw-models/Cargo.toml` — package manifest
- `crates/cw-models/src/lib.rs` — crate root

## How it works

Builds a library.

Modules:

- [[Catalog Module]] — the models CodeWiki offers, as a list inside the program.
- [[Game Module]] — whether a game is running.
- [[Hardware Module]] — what the machine has to run a model with.
- [[Recommend Module]] — which model fits a machine.
- [[Store Module (cw-models)]] — the models on disk.
- [[Worker Module]] — the child process that holds a model, seen from both sides.

Defined at the crate root:

- `ENGINE` — whether this build can run a built-in model.

## Used by

- [[Models Module]] — from `describe`, `unavailable`
- [[Model Module (codewiki)]] — from `run`
- [[Settings Module (cw-server)]] — from `describe`
- [[BuiltinEmbedder]] — from `new`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
