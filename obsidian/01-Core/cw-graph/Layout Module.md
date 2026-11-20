---
tags: [core, cw-graph, module]
---

# Layout Module

> Force-directed layout.

## What it does

Follows the ForceAtlas2 model (Jacomy et al., "ForceAtlas2, a Continuous Graph Layout Algorithm for Handy Network Visualization", 2014): degree-scaled repulsion, linear attraction, gravity and swing-based adaptive speed, with Barnes-Hut approximation of repulsion. Implemented from the paper.

## Source

- `crates/cw-graph/src/layout.rs` — module `cw_graph::layout`

## How it works

1. `analyze` in [[Analyze Module]] calls `LayoutOptions::default`.
2. `analyze` in [[Analyze Module]] calls `force_layout`. `force_layout` calls `QuadTree::build`. `QuadTree::build` calls `QuadTree::cell`, then `QuadTree::insert`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub struct LayoutOptions` — public struct; used by `force_layout`.
- `struct Rng` — SplitMix64: small, fast, and identical on every platform.
- `pub fn force_layout(adjacency: &[Vec<(usize, f64)>], options: LayoutOptions) -> Vec<[f32; 2]>` — compute 2D positions for a symmetric weighted adjacency list.

And 3 more private items with no comment.

## Used by

- [[Analyze Module]] — from `analyze`

## See also

- [[_index]]
- [[cw-graph]]
