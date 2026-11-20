---
tags: [core, cw-models, module]
---

# Catalog Module

> The models CodeWiki offers, as a list inside the program.

## What it does

A model enters the list only after the engine reproduced its reference vectors (spike S5 in `spikes/VERDICTS.md`). The list changes with a release, never at run time: nothing is fetched to learn what exists.

## Source

- `crates/cw-models/src/catalog.rs` — module `cw_models::catalog`

## How it works

1. `entry` and `list` in [[Model Module (codewiki)]], and 3 more, call `all`.
2. `worker` in [[cw-embed-engine]] and `entry` in [[Model Module (codewiki)]], and 2 more, call `find`. `find` calls `all`.
3. `recommend` in [[Recommend Module]] calls `of_tier`. `of_tier` calls `all`.

## Items

- `pub enum Tier` — how much machine a model wants; the recommendation picks by this.
- `pub enum Family` — the architecture, which names the loader the engine uses.
- `pub enum Pooling` — which token's state is the text's vector.
- `pub struct File` — one file of a model, as its repository holds it at the pinned revision.
- [[Entry]] — struct
- `pub fn all() -> &'static [Entry]` — every model, smallest tier first.
- `pub fn find(id: &str) -> Option<&'static Entry>` — the model with this id.
- `pub fn of_tier(tier: Tier) -> Option<&'static Entry>` — the model of a tier.

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
