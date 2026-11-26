---
tags: [core, codewiki-app, module]
---

# Graph Module (codewiki-app)

> Geometry of the dependency graph view.

## What it does

This is the math behind the dependency graph view. The backend lays the nodes out and sends their positions. This module turns those world positions into screen positions, chooses what is worth drawing at the current zoom, and finds the node under the mouse. It holds no UI types, so plain unit tests cover it.

It has to stay fast on graphs of many thousands of nodes. So it caps what it draws: a limited number of edges, a few labels, and density cells when too many nodes are in view.

## Source

- `crates/codewiki-app/src/graph.rs` — module `codewiki_app::graph`

## How it works

`AppView::open_graph` builds a `GraphData` from the backend's JSON with `from_json`, or from `stress`, a synthetic clustered graph used to measure drawing speed. Both call `new`, which finds the largest rank.

On each frame `AppView::graph_view` uses a `Camera`. `Camera::fit` frames the whole graph with a 10 percent margin the first time. `to_screen` and `to_world` convert between the two spaces, `pan` moves by a screen distance, and `zoom_at` zooms while keeping the point under the cursor fixed.

Then the draw step chooses what to draw. `visible_nodes` keeps nodes inside the view plus a small margin. If more than `DETAIL_LIMIT` (2000) are in view, `density_cells` bins them into square cells and draws those instead. Each cell takes the cluster of its highest-ranked node, so important symbols keep their color. `visible_edges` keeps edges with an end in view and samples them evenly down to a limit. `label_candidates` picks the top-ranked visible nodes for labels. `radius` sizes a node from its rank, and `pick` finds the nearest node under the cursor, using the same radius plus 4 pixels.

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

## Gotchas

- `from_json` is forgiving. A missing or wrong field becomes an empty string or zero, and an edge whose index is out of range, or that links a node to itself, is dropped without a message.
- Because of that, the edge count shown can differ from the backend's.
- Zoom is clamped between `MIN_ZOOM` and `MAX_ZOOM`, both in `Camera::fit` and in `zoom_at`.
- `visible_edges` with a limit of zero returns nothing. Over the limit it samples by even steps, so it shows overall shape, not every edge.
- `Camera::fit` on an empty graph returns zoom 1 at the origin.
- `max_rank` is never below a tiny positive value, so `radius` cannot divide by zero.

## See also

- [[_index]]
- [[codewiki-app]]
