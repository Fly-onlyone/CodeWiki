---
tags: [core, cw-graph, module]
---

# Layout Module

> Force-directed layout.

## What it does

This module gives every node a 2D position so the graph can be drawn. Linked nodes pull toward each other, all nodes push apart, and a mild pull to the centre keeps separate parts from drifting away. This is the ForceAtlas2 idea. The result is deterministic: the same graph and seed always give the same positions, so a stored layout does not jump between runs.

## Source

- `crates/cw-graph/src/layout.rs` — module `cw_graph::layout`

## How it works

`force_layout` starts with seeded random positions from `Rng`, a small SplitMix64 generator that gives the same numbers on every platform. A node's mass is its number of neighbours plus one. It then repeats for `iterations` rounds (120 by default):

1. Build a `QuadTree` of all positions. `repulsion` walks it and treats far groups of nodes as one lump, using the opening angle `theta`. This is the Barnes-Hut shortcut and avoids comparing every pair.
2. Add gravity toward the origin and a linear pull along each edge, scaled by its weight.
3. Compute swing (how much a force changed) and traction (how steady it is) and set a global speed from them. Speed may rise by at most half per round.
4. Move each node by its force times a per-node factor that shrinks when its force swings, and is capped so no step exceeds 10 units.

An empty graph returns nothing and a one-node graph returns the origin.

## Items

- `pub struct LayoutOptions` — public struct; used by `force_layout`.
- `struct Rng` — SplitMix64: small, fast, and identical on every platform.
- `pub fn force_layout(adjacency: &[Vec<(usize, f64)>], options: LayoutOptions) -> Vec<[f32; 2]>` — compute 2D positions for a symmetric weighted adjacency list.

And 3 more private items with no comment.

## Used by

- [[Analyze Module]] — from `analyze`

## Gotchas

- Only the seed makes it random. Change `seed`, `iterations`, `scaling`, `gravity` or `theta` and every stored position changes.
- Points on the same spot would split the tree forever, so the tree stops at depth 40 and keeps them in one cell.
- The input must be a symmetric adjacency list, as `Graph::undirected` gives.
- Positions are `f32`; the step limit and the speed floor of 1e-4 keep values finite.

## See also

- [[_index]]
- [[cw-graph]]
