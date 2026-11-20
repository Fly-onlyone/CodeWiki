---
tags: [core, cw-models, module]
---

# Catalog Module

> The models CodeWiki offers, as a list inside the program.

## Source

- `crates/cw-models/src/catalog.rs` — module `cw_models::catalog`

## How it works

Main items:

- [[Entry]] — struct
- `all` — every model, smallest tier first.
- `Pooling` — which token's state is the text's vector.
- `find` — the model with this id.
- `Tier` — how much machine a model wants; the recommendation picks by this.
- `Family` — the architecture, which names the loader the engine uses.
- `File` — one file of a model, as its repository holds it at the pinned revision.
- `of_tier` — the model of a tier.

## Depends on

- [[Entry]] — uses `Entry`

## Used by

- [[Model Module (codewiki)]] — from `entry`, `list`, `recommend`
- [[codewiki]] — from `install_by_hand`, `recommends_a_model_for_this_machine_with_its_reason`
- [[Models Module]] — from `describe`, `entry`
- [[cw-embed-engine]] — from `worker`
- [[cw-server]] — from `install_model`
- [[Recommend Module]] — from `recommend`
- [[BuiltinEmbedder]] — from `new`
- [[Engine]] — from `Engine`

## See also

- [[_index]]
- [[cw-models]]
