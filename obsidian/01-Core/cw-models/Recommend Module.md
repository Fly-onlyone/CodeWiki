---
tags: [core, cw-models, module]
---

# Recommend Module

> Which model fits a machine.

## Source

- `crates/cw-models/src/recommend.rs` — module `cw_models::recommend`

## How it works

A fixed rule, so that the same machine always gets the same answer and the answer can be explained in one line.

Main items:

- [[Device]] — what runs the model.
- `Recommendation` — struct
- `recommend` — the model for this machine.
- `GB` — const
- `BALANCED_CORES` — const
- `BALANCED_RAM` — memory and cores from which the balanced model is recommended.
- `DISK_MARGIN` — disk space that must stay free after the download.
- `LARGE_VRAM` — video memory from which the large model is recommended on the card.

## Depends on

- [[Catalog Module]] — uses `of_tier`
- [[Entry]] — uses `size`
- [[Hardware]] — uses `Hardware`
- [[Device]] — uses `Device`

## See also

- [[_index]]
- [[cw-models]]
