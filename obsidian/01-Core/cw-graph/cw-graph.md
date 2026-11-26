---
tags: [core, cw-graph]
---

# cw-graph

> Graph construction, metrics, community detection and layout for CodeWiki.

## What it does

`cw-graph` is the graph math of CodeWiki. It builds a directed weighted graph from the resolved edges of a repository, then measures it: importance, cycles, bridges, shortest routes, communities and a 2D layout for drawing. It has no knowledge of files or databases and does no input or output, so it is small and easy to test. Every algorithm is deterministic: the same input gives the same output, so generated notes and stored layouts do not churn.

## Source

- `crates/cw-graph/Cargo.toml` — package manifest
- `crates/cw-graph/src/lib.rs` — crate root

## How it works

The crate root only re-exports. [[Graph Module]] defines the `Graph`. [[Metrics Module]] has `pagerank`, `strongly_connected_components`, `betweenness` and `shortest_path`. [[Community Module]] has `louvain`. [[Layout Module]] has `force_layout`. The metrics read the `Graph`. Community detection and layout read the symmetric number lists from `Graph::undirected`. Callers such as the analyze step, the query code and the refactor planner pull these pieces together.

## Items

Modules:

- [[Community Module]] — community detection.
- [[Graph Module]] — directed weighted graph over string-named nodes.
- [[Layout Module]] — Force-directed layout.
- [[Metrics Module]] — centrality and connectivity metrics.

## Gotchas

- Determinism is a rule here: names are sorted, ties go to the lowest index, and the layout uses its own random generator with a fixed seed. Do not add an algorithm that depends on hash order or the system clock.
- Results are numbers by node index. Keep the `Graph` around to turn them back into names.
- Large graphs trade exactness for cost in `betweenness`, which samples start nodes.

## See also

- [[_index]]
- [[Crate Dependency Flow]]
