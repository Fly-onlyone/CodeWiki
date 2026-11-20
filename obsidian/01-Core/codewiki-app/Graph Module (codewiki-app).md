---
tags: [core, codewiki-app, module]
---

# Graph Module (codewiki-app)

> Geometry of the dependency graph view.

## What it does

No UI types, so it is unit-tested.

Node positions come from the backend's layout. This module maps them to the screen, decides what is worth drawing at the current zoom, and finds the node under the cursor.

## Source

- `crates/codewiki-app/src/graph.rs` — module `codewiki_app::graph`

## How it works

1. `AppView::open_graph` in [[codewiki-app]] calls `GraphData::from_json`. `GraphData::from_json` calls `GraphData::new`.
2. `AppView::open_graph` in [[codewiki-app]] calls `GraphData::stress`. `GraphData::stress` calls `GraphData::new`.
3. `AppView::graph_view` in [[codewiki-app]] calls `Camera::fit`.
4. `AppView::graph_view` in [[codewiki-app]] calls `radius`.
5. `AppView::graph_view` in [[codewiki-app]] calls `pick`. `pick` calls `radius`.

5 more entry points: `visible_nodes`, `visible_edges`, `cell_size`, `density_cells`, `label_candidates`.

## Items

- `pub struct GraphNode` — public struct; used by `GraphData`, `GraphData::from_json`, `GraphData::new`, `GraphData::stress`.
- `pub struct GraphData` — public struct; used by `AppView`, `Camera::fit`, `density_cells`, `label_candidates` and 3 more.
- `pub struct Camera` — maps world coordinates to the view. `(cx, cy)` is the world point shown at the centre of the view.
- `pub const MIN_ZOOM: f32` — public constant; used by `Camera::fit`, `Camera::zoom_at`.
- `pub const MAX_ZOOM: f32` — public constant; used by `Camera::fit`, `Camera::zoom_at`.
- `pub fn radius(rank: f32, max_rank: f32, zoom: f32) -> f32` — drawn radius of a node: more depended-upon nodes are larger, and nodes grow a little when zoomed in.
- `pick` — index of the node under the cursor, preferring the nearest.
- `visible_nodes` — screen positions of the nodes inside the view, with their index.
- `visible_edges` — edges worth drawing: at least one end in view, capped at `limit` by even sampling so a dense view stays fast and still shows overall structure.
- `pub const DETAIL_LIMIT: usize` — above this many nodes in view, the view draws density cells instead of every node. Drawing cost then depends on the view size, not the graph size.
- `const TARGET_CELLS: f32` — cells drawn in the overview; chosen so a frame stays well under 16 ms.
- `pub struct Cell` — one overview cell: screen position of its corner, the cluster most of its nodes belong to, and how many nodes it holds.
- `pub fn cell_size(width: f32, height: f32) -> f32` — side of an overview cell for a view of this size.
- `pub fn density_cells(data: &GraphData, camera: &Camera, width: f32, height: f32) -> Vec<Cell>` — bin the nodes in view into square screen cells. Each cell takes the cluster of its highest-ranked node, so important symbols keep their color.
- `label_candidates` — indexes of the highest-ranked visible nodes, for labels.

## Used by

- [[Diagram View Module]] — from `actual`, `clamp`, `fit`, `frame`
- [[codewiki-app]] — from `AppView`, `DiagramViewer`, `diagram_viewer_view`, `graph_view`

## See also

- [[_index]]
- [[codewiki-app]]
