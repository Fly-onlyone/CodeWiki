---
tags: [core, cw-graph, module]
---

# Community Module

> Community detection.

## Source

- `crates/cw-graph/src/community.rs` — module `cw_graph::community`

## How it works

Main items:

- `louvain` — partition a symmetric weighted adjacency list.
- `MAX_SWEEPS` — upper bound on local-moving sweeps per level; real graphs settle far sooner.
- `local_moving` — one level of greedy modularity optimization.
- `renumber` — map arbitrary ids to `0..count` in order of first appearance.

## Used by

- [[Analyze Module]] — from `analyze`

## See also

- [[_index]]
- [[cw-graph]]
