---
tags: [core, cw-graph, module]
---

# Metrics Module

> Centrality and connectivity metrics.

## What it does

This module answers questions about the shape of the dependency [[Graph]]. Which nodes matter most (`pagerank`)? Which nodes sit on many routes between others (`betweenness`)? Which nodes form a cycle (`strongly_connected_components`)? What is the shortest chain from one node to another (`shortest_path`)? The analysis and the dependency-path query use the answers.

## Source

- `crates/cw-graph/src/metrics.rs` — module `cw_graph::metrics`

## How it works

`pagerank` repeats power iteration until the total change falls under a tolerance. An edge `a -> b` means a depends on b, so rank flows to what is depended on most. Rank held by nodes with no outgoing edges is spread evenly. Scores sum to 1. `strongly_connected_components` is Tarjan's algorithm with its own stack, not recursion, so a deep graph cannot overflow the call stack. A component with more than one node is a cycle. `betweenness` is Brandes' method on edge counts only, ignoring weights. `shortest_path` is a breadth-first search that counts edges and returns the node numbers from start to end, or `None`.

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

## Gotchas

- For a graph larger than `max_pivots` nodes, `betweenness` runs from evenly spaced start nodes and scales the result. It is an estimate, kept cheap for repositories of 50 thousand nodes.
- `betweenness` and `shortest_path` ignore edge weights; `pagerank` uses them.
- Component ids and results are plain numbers by node index, not names. Map them with `Graph::id` and `nodes`.
- `shortest_path` returns `None` for a node number out of range as well as for no route.

## See also

- [[_index]]
- [[cw-graph]]
