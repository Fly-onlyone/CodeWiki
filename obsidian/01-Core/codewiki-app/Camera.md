---
tags: [core, codewiki-app]
---

# Camera

> Maps world coordinates to the view.

## Source

- `crates/codewiki-app/src/graph.rs` — definition of `Camera`

## How it works

Declared as `pub struct Camera`.

`(cx, cy)` is the world point shown at the centre of the view.

Public API:

- `pub fn fit(data: &GraphData, width: f32, height: f32) -> Self` — show the whole graph with a small margin.
- `pub fn pan(&mut self, dx: f32, dy: f32)` — move the view by a screen-space distance.
- `pub fn zoom_at(&mut self, sx: f32, sy: f32, factor: f32, width: f32, height: f32)` — zoom by `factor`, keeping the world point under `(sx, sy)` in place.
- `pub fn to_screen(self, x: f32, y: f32, width: f32, height: f32) -> (f32, f32)`
- `pub fn to_world(self, sx: f32, sy: f32, width: f32, height: f32) -> (f32, f32)`

## Depends on

- [[Graph Module (codewiki-app)]] — uses `MAX_ZOOM`, `MIN_ZOOM`
- [[GraphData]] — uses `GraphData`

## Used by

- [[Graph Module (codewiki-app)]] — from `density_cells`, `pick`, `visible_edges`, `visible_nodes`
- [[codewiki-app]] — from `AppView`, `graph_view`

## See also

- [[_index]]
- [[Graph Module (codewiki-app)]]
