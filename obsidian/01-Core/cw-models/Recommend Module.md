---
tags: [core, cw-models, module]
---

# Recommend Module

> Which model fits a machine.

## What it does

A fixed rule, so that the same machine always gets the same answer and the answer can be explained in one line.

## Source

- `crates/cw-models/src/recommend.rs` — module `cw_models::recommend`

## How it works

What the functions of this module call:

1. `recommend` calls `gb`, then `of_tier` in [[Catalog Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const LARGE_VRAM: u64` — video memory from which the large model is recommended on the card.
- `const BALANCED_RAM: u64` — memory and cores from which the balanced model is recommended. A machine sold with 8 GB reports a little less, so the line is drawn below it.
- `const LEAST_FREE_RAM: u64` — free memory below which no model is recommended.
- `const DISK_MARGIN: u64` — disk space that must stay free after the download.
- [[Device]] — what runs the model.
- `pub struct Recommendation` — public struct; used by `recommend`.
- `fn gb(bytes: u64) -> String` — gigabytes with one decimal, as Windows shows them.
- `pub fn recommend(hardware: &Hardware, cuda: bool) -> Recommendation` — the model for this machine. `cuda` says whether this installation can run a model on an NVIDIA card at all (its CUDA worker is present).

And 2 more private items with no comment.

## Depends on

- [[Catalog Module]] — uses `of_tier`
- [[Entry]] — uses `size`
- [[Hardware]] — uses `Hardware`
- [[Device]] — uses `Device`

## See also

- [[_index]]
- [[cw-models]]
