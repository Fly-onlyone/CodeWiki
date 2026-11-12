---
tags: [core, cw-graph]
---

# Graph

> `Graph` struct defined in `cw_graph::graph` with 7 methods.

## Source

- `crates/cw-graph/src/graph.rs` — definition of `Graph`

## How it works

Declared as `pub struct Graph`.

Public API:

- `pub fn from_weighted<I, N>(nodes: N, edges: I) -> Self where N: IntoIterator<Item = String>, I: IntoIterator<Item = (String, String, f64)>,` — build from weighted pairs.
- `pub fn from_edges( nodes: impl IntoIterator<Item = String>, edges: &[Edge], kinds: &[EdgeKind], ) -> Self` — build from resolved edges, keeping only `kinds` and weighting each edge by its confidence.
- `pub fn undirected(&self) -> Vec<Vec<(usize, f64)>>` — symmetric adjacency: weights of both directions summed per neighbor.
- `pub fn len(&self) -> usize`
- `pub fn id(&self, name: &str) -> Option<usize>`
- `pub fn is_empty(&self) -> bool`

## Depends on

- [[Model Module]] — uses `Edge`, `EdgeKind`
- [[Confidence]] — uses `weight`

## Used by

- [[codewiki-app]] — from `edit_view`, `graph_view`, `palette_view`, `render`
- [[Metrics Module]] — from `betweenness`, `pagerank`, `shortest_path`, `strongly_connected_components`
- [[Java Module]] — from `enum_constant`, `method`, `type_decl`
- [[Analyze Module]] — from `analyze`, `dependency_graph`
- [[Typescript Module]] — from `class`
- [[Query Module]] — from `dependency_path`
- [[Refactor Module]] — from `break_cycles`

## See also

- [[_index]]
- [[Graph Module]]
