---
tags: [core, codewiki-app]
---

# GraphData

> `GraphData` struct defined in `codewiki_app::graph` with 3 methods.

## Source

- `crates/codewiki-app/src/graph.rs` — definition of `GraphData`

## How it works

Declared as `pub struct GraphData`.

Public API:

- `pub fn stress(node_count: usize) -> Self` — A synthetic clustered graph for measuring drawing speed.
- `pub fn from_json(value: &Value) -> Self`

## Depends on

- [[Graph Module (codewiki-app)]] — uses `GraphNode`

## Used by

- [[Graph Module (codewiki-app)]] — from `density_cells`, `label_candidates`, `pick`, `visible_edges`
- [[codewiki-app]] — from `AppView`, `open_graph`
- [[Camera]] — from `fit`

## See also

- [[_index]]
- [[Graph Module (codewiki-app)]]
