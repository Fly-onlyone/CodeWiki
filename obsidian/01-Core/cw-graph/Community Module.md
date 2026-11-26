---
tags: [core, cw-graph, module]
---

# Community Module

> Community detection.

## What it does

This module groups the nodes of the code graph into communities: sets of files or symbols that link to each other more than to the rest. CodeWiki uses the groups to show which parts of a repository belong together. It uses the Louvain method, which picks groups that raise a score called modularity. Nodes are visited in index order and ties go to the lowest community id, so the same graph always gives the same groups.

## Source

- `crates/cw-graph/src/community.rs` — module `cw_graph::community`

## How it works

`louvain` takes a symmetric weighted adjacency list, as made by `Graph::undirected` in [[Graph]]. It repeats two steps.

1. `local_moving` starts with each node alone. It sweeps the nodes in order and moves each one to the neighbouring community with the best modularity gain, until a sweep moves nothing or 100 sweeps are done.
2. If something moved, `renumber` gives the communities ids from 0, and the graph is collapsed to one node per community. Weight inside a community becomes a self loop, so node strength stays the same at the next level.

It stops when nothing moves or one community is left. The final ids come from `renumber`, so they count up in order of each community's lowest node. [[Analyze Module]] calls it.

## Items

- `const MAX_SWEEPS: usize` — upper bound on local-moving sweeps per level; real graphs settle far sooner.
- `pub fn louvain(adjacency: &[Vec<(usize, f64)>]) -> Vec<usize>` — partition a symmetric weighted adjacency list. Returns a community id per node, numbered in order of each community's lowest node index.
- `fn local_moving(adj: &[Vec<(usize, f64)>]) -> (Vec<usize>, bool)` — one level of greedy modularity optimization.
- `fn renumber(ids: &[usize]) -> (Vec<usize>, usize)` — map arbitrary ids to `0..count` in order of first appearance.

## Used by

- [[Analyze Module]] — from `analyze`

## Gotchas

- The input must be symmetric. A directed list gives wrong gains.
- A move must beat the best gain by more than 1e-12, so tiny float differences cannot cause flipping.
- A graph with no weight at all returns every node alone.
- Isolated nodes stay in their own community.
- `MAX_SWEEPS` is a safety cap; real graphs stop long before it.

## See also

- [[_index]]
- [[cw-graph]]
