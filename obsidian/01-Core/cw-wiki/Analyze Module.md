---
tags: [core, cw-wiki, module]
---

# Analyze Module

> Graph metrics for a scanned repository.

## Source

- `crates/cw-wiki/src/analyze.rs` — module `cw_wiki::analyze`

## How it works

1. `reindex` in [[Service Module]] calls `analyze`. `analyze` calls `dependency_graph`, then `pagerank`, then `betweenness`, then `strongly_connected_components` (all in [[Metrics Module]]), then `louvain` in [[Community Module]], then `force_layout` in [[Layout Module]], and 1 more. `dependency_graph` calls `Graph::from_edges` in [[Graph]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

## See also

- [[_index]]
- [[cw-wiki]]
