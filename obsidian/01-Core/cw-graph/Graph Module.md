---
tags: [core, cw-graph, module]
---

# Graph Module

> Directed weighted graph over string-named nodes.

## What it does

This module holds the one data structure the graph crate works on: [[Graph]], a directed weighted graph whose nodes are named by strings. The names are symbol names or file paths. Everything else in the crate (ranking, cycles, communities, layout) reads a `Graph` or the number lists made from it.

## Source

- `crates/cw-graph/src/graph.rs` — module `cw_graph::graph`

## How it works

A graph is built with `from_weighted` or `from_edges`. The second one keeps only the edge kinds asked for and uses each edge's confidence as its weight, so a weakly resolved link counts less. The build step sorts and removes duplicate node names, then gives each name a number, so the numbering depends only on the names. Parallel edges add their weights and self loops are dropped. The graph keeps an outgoing list and an incoming list per node, both sorted. `undirected` makes a symmetric list for the community and layout code, adding the weights of both directions.

## Items

- [[Graph]] — struct

## Gotchas

- Node numbers are positions in the sorted name list. Adding a node can shift the numbers of others, so never store them between runs. Store names.
- Self loops are dropped without a warning, so a function that calls itself leaves no trace.
- An edge with a name not in `nodes` still creates that node.

## See also

- [[_index]]
- [[cw-graph]]
