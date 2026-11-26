---
tags: [core, cw-graph]
---

# Graph

> `Graph` struct defined in `cw_graph::graph` with 7 methods.

## What it does

`Graph` is the dependency graph of a repository in memory. Each node is a symbol or file; an edge `a -> b` means a depends on b. Each edge has a weight. The metrics in [[Metrics Module]], the cycle breaking in [[Refactor Module]] and the path search in [[Query Module]] all run on it.

## Source

- `crates/cw-graph/src/graph.rs` — definition of `Graph`

## How it works

`from_edges` filters the resolved edges by kind and gives each a weight from its confidence, then calls `from_weighted`. That sorts the node names, merges parallel edges by adding weights, drops self loops, and fills two lists per node: `out` (targets) and `inc` (sources). `id` finds a node number by name, `len` and `edge_count` count nodes and edges, and `undirected` makes the symmetric view for community and layout work.

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

## Gotchas

- Node numbers come from sorted names, so they change when nodes are added.
- `out` and `inc` are public fields, but `index` is private; build graphs only through the constructors so the three stay in step.
- The graph owns no edge kinds. Which relations count is decided by the `kinds` argument at build time, so the same edges can give different graphs.

## See also

- [[_index]]
- [[Graph Module]]
