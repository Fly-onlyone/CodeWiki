---
tags: [core, cw-wiki, module]
---

# Analyze Module

> Graph metrics for a scanned repository.

## What it does

After a repository is scanned, this module turns the symbol edges into numbers per symbol: how central it is, whether it sits in a loop, which cluster it belongs to, and where to draw it. Those numbers feed health scores, dead-code checks, refactoring hints and the graph view in the app.

## Source

- `crates/cw-wiki/src/analyze.rs` — module `cw_wiki::analyze`

## How it works

`dependency_graph` builds a directed graph whose nodes are every symbol's qualified name. Only edges of kind Calls, References and Implements count as dependencies (`DEPENDENCY_KINDS`).

`analyze` then runs these steps on that graph:

- `pagerank` with damping 0.85, up to 100 rounds.
- `betweenness`, sampled from 256 source nodes.
- `strongly_connected_components`, to find dependency loops.
- `louvain` community detection on the undirected copy.
- `force_layout` on the undirected copy, giving x and y.

Each result array is indexed by node position. The function zips them into one `NodeMetrics` per node and returns the list. `metrics_map` turns that list into a lookup by name.

## Items

- `pub const DEPENDENCY_KINDS: [EdgeKind; 3]` — edge kinds that express "depends on" between symbols.
- `const BETWEENNESS_PIVOTS: usize` — sources sampled for betweenness on large graphs.
- `pub fn dependency_graph(facts: &RepoFacts) -> Graph` — build the symbol dependency graph.
- `pub fn analyze(facts: &RepoFacts) -> Vec<(String, NodeMetrics)>` — compute every per-node metric and a layout, sorted by node name.
- `pub fn metrics_map(metrics: &[(String, NodeMetrics)]) -> HashMap<&str, &NodeMetrics>` — public function, returns `HashMap<&str, &NodeMetrics>`.

## Depends on

- [[Metrics Module]] — uses `betweenness`, `pagerank`, `strongly_connected_components`
- [[Graph]] — uses `Graph`, `from_edges`, `undirected`
- [[Layout Module]] — uses `default`, `force_layout`
- [[Model Module]] — uses `EdgeKind`
- [[Community Module]] — uses `louvain`
- [[Store Module]] — uses `NodeMetrics`
- [[Scan Module]] — uses `RepoFacts`

## Used by

- [[Health Module]] — from `dead_code`, `file_health`
- [[Query Module]] — from `dependency_path`, `related`
- [[Api Module]] — from `graph`
- [[Refactor Module]] — from `load`
- [[Service Module]] — from `reindex`
- [[RenderCtx]] — from `new`

## Gotchas

- Betweenness is sampled (256 pivots), so on large graphs it is an estimate, not exact.
- Community and layout use the undirected graph. Pagerank, betweenness and loops use the directed one.
- Symbols with no dependency edges are still nodes, so they get metrics and a layout position.

## See also

- [[_index]]
- [[cw-wiki]]
