---
tags: [core, cw-graph, module]
---

# Metrics Module

> Centrality and connectivity metrics.

## Source

- `crates/cw-graph/src/metrics.rs` — module `cw_graph::metrics`

## How it works

1. `analyze` in [[Analyze Module]] and `break_cycles` in [[Refactor Module]] call `strongly_connected_components`.
2. `analyze` in [[Analyze Module]] calls `pagerank`.
3. `analyze` in [[Analyze Module]] calls `betweenness`.
4. `dependency_path` in [[Query Module]] calls `shortest_path`.

## Items

- `pub fn pagerank(g: &Graph, damping: f64, max_iterations: usize, tolerance: f64) -> Vec<f64>` — weighted PageRank by power iteration. Scores sum to 1. An edge `a -> b` means "a depends on b", so rank flows to the things most depended upon. Rank held by nodes with no outgoing edges is spread evenly.
- `pub fn strongly_connected_components(g: &Graph) -> Vec<usize>` — Tarjan's algorithm, iterative. Returns a component id per node; ids are numbered in order of each component's lowest node index.
- `pub fn betweenness(g: &Graph, max_pivots: usize) -> Vec<f64>` — Brandes betweenness centrality on the unweighted directed graph. Exact when the graph has at most `max_pivots` nodes. Larger graphs use `max_pivots` evenly spaced source nodes and scale the result, which keeps the cost bounded for 50K-node repositories.
- `pub fn shortest_path(g: &Graph, from: usize, to: usize) -> Option<Vec<usize>>` — shortest directed path from `from` to `to` by edge count, inclusive.

## Depends on

- [[Graph]] — uses `Graph`

## Used by

- [[Analyze Module]] — from `analyze`
- [[Query Module]] — from `dependency_path`
- [[Refactor Module]] — from `break_cycles`

## See also

- [[_index]]
- [[cw-graph]]
