---
tags: [core, cw-graph, module]
---

# Community Module

> Community detection.

## What it does

Implements the Louvain method (Blondel et al., "Fast unfolding of communities in large networks", 2008) from the paper. Nodes are visited in index order and ties go to the lowest community id, so results are stable.

## Source

- `crates/cw-graph/src/community.rs` — module `cw_graph::community`

## How it works

1. `analyze` in [[Analyze Module]] calls `louvain`. `louvain` calls `local_moving`, then `renumber`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const MAX_SWEEPS: usize` — upper bound on local-moving sweeps per level; real graphs settle far sooner.
- `pub fn louvain(adjacency: &[Vec<(usize, f64)>]) -> Vec<usize>` — partition a symmetric weighted adjacency list. Returns a community id per node, numbered in order of each community's lowest node index.
- `fn local_moving(adj: &[Vec<(usize, f64)>]) -> (Vec<usize>, bool)` — one level of greedy modularity optimization.
- `fn renumber(ids: &[usize]) -> (Vec<usize>, usize)` — map arbitrary ids to `0..count` in order of first appearance.

## Used by

- [[Analyze Module]] — from `analyze`

## See also

- [[_index]]
- [[cw-graph]]
