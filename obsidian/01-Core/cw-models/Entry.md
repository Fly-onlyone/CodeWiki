---
tags: [core, cw-models]
---

# Entry

> `Entry` struct defined in `cw_models::catalog` with 2 methods.

## Source

- `crates/cw-models/src/catalog.rs` — definition of `Entry`

## Items

Declared as `pub struct Entry`.

Members:

- `pub fn size(&self) -> u64` — bytes to download.
- `pub fn embedder_name(&self) -> String` — the name stored with each vector. It holds the revision, so that a release that moves a model to other weights embeds everything again.

## Depends on

- [[Catalog Module]] — uses `Family`, `File`, `Pooling`, `Tier`

## Used by

- [[Store]] — from `dir`, `have`, `install`, `installed`
- [[Model Module (codewiki)]] — from `entry`, `install`, `list`, `recommend`
- [[Models Module]] — from `describe`, `entry`, `run`, `switch`
- [[Catalog Module]] — from `CATALOG`, `all`, `find`, `of_tier`
- [[BuiltinEmbedder]] — from `BuiltinEmbedder`, `name`
- [[Recommend Module]] — from `recommend`
- [[Engine]] — from `load`

## See also

- [[_index]]
- [[Catalog Module]]
