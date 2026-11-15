---
tags: [core, codewiki-app, module]
---

# Graph Module (codewiki-app)

> Geometry of the dependency graph view.

## Source

- `crates/codewiki-app/src/graph.rs` — module `codewiki_app::graph`

## How it works

No UI types, so it is unit-tested.

Main items:

- `GraphData` — struct
- `Camera` — maps world coordinates to the view.
- `GraphNode` — struct
- `MAX_ZOOM` — const
- `MIN_ZOOM` — const
- `radius` — drawn radius of a node: more depended-upon nodes are larger, and nodes grow a little when zoomed in.
- `cell_size` — side of an overview cell for a view of this size.
- `Cell` — one overview cell: screen position of its corner, the cluster most of its nodes belong to, and how many nodes it holds.

## Used by

- [[codewiki-app]] — from `AppView`, `graph_view`, `open_graph`

## See also

- [[_index]]
- [[codewiki-app]]
