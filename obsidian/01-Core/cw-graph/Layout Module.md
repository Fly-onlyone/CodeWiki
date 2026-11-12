---
tags: [core, cw-graph, module]
---

# Layout Module

> Force-directed layout.

## Source

- `crates/cw-graph/src/layout.rs` — module `cw_graph::layout`

## How it works

Main items:

- `LayoutOptions` — struct
- `force_layout` — compute 2D positions for a symmetric weighted adjacency list.
- `QuadTree` — struct
- `NO_CHILD` — const
- `Cell` — struct
- `Rng` — SplitMix64: small, fast, and identical on every platform.

## Used by

- [[Analyze Module]] — from `analyze`

## See also

- [[_index]]
- [[cw-graph]]
