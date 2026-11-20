---
tags: [core, cw-models, module]
---

# Hardware Module

> What the machine has to run a model with.

## Source

- `crates/cw-models/src/hardware.rs` — module `cw_models::hardware`

## How it works

1. `recommend` in [[Model Module (codewiki)]] calls `probe`. `probe` calls `nvidia_card`, then `disk_free`.
2. `cuda_usable` in [[Worker Module]] calls `nvidia_card`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- [[Nvidia]] — the NVIDIA card a model can run on.
- [[Hardware]] — struct
- `pub fn probe(data_dir: &Path) -> Hardware` — read the machine. Nothing here fails: a reading that cannot be taken is reported as absent or zero, and the recommendation is cautious about it.
- `fn nvidia() -> Option<Nvidia>` — the NVIDIA card with the most video memory. The driver's library is loaded at run time, so a machine without the driver simply has no card.
- `pub fn nvidia_card() -> Option<&'static Nvidia>` — the machine's NVIDIA card, read once for this process: asking the driver loads its library, and the answer does not change while a program runs. The free video memory in it is as old as the first call.
- `fn disk_free(path: &Path) -> Option<u64>` — free bytes on the drive whose mount point is the longest start of `path`.

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
