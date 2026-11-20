---
tags: [core, cw-wiki, module]
---

# Analyze Module

> Graph metrics for a scanned repository.

## Source

- `crates/cw-wiki/src/analyze.rs` — module `cw_wiki::analyze`

## How it works

Main items:

- `DEPENDENCY_KINDS` — edge kinds that express "depends on" between symbols.
- `analyze` — compute every per-node metric and a layout, sorted by node name.
- `dependency_graph` — build the symbol dependency graph.
- `metrics_map` — function
- `BETWEENNESS_PIVOTS` — sources sampled for betweenness on large graphs.

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
