---
tags: [core, cw-graph]
---

# Graph

> `Graph` struct defined in `cw_graph::graph` with 7 methods.

## Source

- `crates/cw-graph/src/graph.rs` — definition of `Graph`

## How it works

1. `dependency_graph` in [[Analyze Module]] and `dependency_path` in [[Query Module]] call `from_edges`. `from_edges` calls `from_weighted`.
2. `break_cycles` in [[Refactor Module]] calls `from_weighted`. `from_weighted` calls `len`.

## Items

Declared as `pub struct Graph`.

Members:

- `from_weighted` — build from weighted pairs. Parallel edges are merged by summing weights; self loops are dropped. Nodes are sorted by name for determinism.
- `from_edges` — build from resolved edges, keeping only `kinds` and weighting each edge by its confidence.
- `pub fn len(&self) -> usize` — public method, returns `usize`; called by `Graph::from_weighted`, `Graph::undirected`.
- `pub fn is_empty(&self) -> bool` — public method, returns `bool`.
- `pub fn edge_count(&self) -> usize` — public method, returns `usize`.
- `pub fn id(&self, name: &str) -> Option<usize>` — public method, returns `Option<usize>`.
- `pub fn undirected(&self) -> Vec<Vec<(usize, f64)>>` — symmetric adjacency: weights of both directions summed per neighbor.

## Depends on

- [[Model Module]] — uses `Edge`, `EdgeKind`

## Used by

- [[Metrics Module]] — from `betweenness`, `pagerank`, `shortest_path`, `strongly_connected_components`
- [[Analyze Module]] — from `analyze`, `dependency_graph`
- [[Query Module]] — from `dependency_path`
- [[Refactor Module]] — from `break_cycles`

## See also

- [[_index]]
- [[Graph Module]]
