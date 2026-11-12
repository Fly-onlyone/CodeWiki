---
tags: [core, cw-graph, module]
---

# Metrics Module

> Centrality and connectivity metrics.

## Source

- `crates/cw-graph/src/metrics.rs` — module `cw_graph::metrics`

## How it works

Main items:

- `strongly_connected_components` — Tarjan's algorithm, iterative.
- `shortest_path` — shortest directed path from `from` to `to` by edge count, inclusive.
- `betweenness` — Brandes betweenness centrality on the unweighted directed graph.
- `pagerank` — weighted PageRank by power iteration.

## Depends on

- [[Graph]] — uses `Graph`

## Used by

- [[Analyze Module]] — from `analyze`
- [[Query Module]] — from `dependency_path`
- [[Refactor Module]] — from `break_cycles`

## See also

- [[_index]]
- [[cw-graph]]
