---
tags: [core, cw-models, module]
---

# Hardware Module

> What the machine has to run a model with.

## Source

- `crates/cw-models/src/hardware.rs` — module `cw_models::hardware`

## How it works

Main items:

- [[Nvidia]] — the NVIDIA card a model can run on.
- [[Hardware]] — struct
- `nvidia_card` — the machine's NVIDIA card, read once for this process: asking the driver loads its library, and the answer does not change while a program runs.
- `probe` — read the machine.
- `disk_free` — free bytes on the drive whose mount point is the longest start of `path`.
- `nvidia` — the NVIDIA card with the most video memory.

## Depends on

- [[Hardware]] — uses `Hardware`
- [[Nvidia]] — uses `Nvidia`

## Used by

- [[Model Module (codewiki)]] — from `recommend`
- [[Worker Module]] — from `cuda_usable`
- [[BuiltinEmbedder]] — from `new`
- [[Host]] — from `default`

## See also

- [[_index]]
- [[cw-models]]
