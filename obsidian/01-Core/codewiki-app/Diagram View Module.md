---
tags: [core, codewiki-app, module]
---

# Diagram View Module

> Where an enlarged diagram sits in the diagram viewer.

## What it does

A note narrows a wide diagram to its column, which leaves the labels too small to read. A click on the diagram opens it in the viewer, over the window, where the wheel zooms and a drag moves it.

The viewer looks at the diagram through a `Camera`, as the graph view looks at the graph: the world is the diagram, one unit per pixel of its natural size, with the origin at its top left corner. All sizes here are in design units. No view types, so it is unit-tested.

## Source

- `crates/codewiki-app/src/diagram_view.rs` — module `codewiki_app::diagram_view`

## How it works

1. `AppView::diagram_viewer_view` in [[codewiki-app]] calls `fit`. `fit` calls `placed`.
2. `AppView::diagram_viewer_view` in [[codewiki-app]] calls `actual`. `actual` calls `placed`.
3. `AppView::diagram_viewer_view` and `AppView::zoom_diagram` in [[codewiki-app]] call `zoom_by`.
4. `AppView::move_diagram` in [[codewiki-app]] calls `clamp`.
5. `AppView::diagram_viewer_view` in [[codewiki-app]] calls `frame`.

2 more entry points: `percent`, `png_size`.

## Items

- `pub const MIN_ZOOM: f32` — the smallest and the largest the viewer draws a diagram, as a share of its natural size. Past four times, the picture is only blurred.
- `pub const MAX_ZOOM: f32` — public constant; used by `placed`, `zoom_by`.
- `pub const STEP: f32` — how much one press of a zoom button, or of `+` and `-`, changes the zoom.
- `pub const WHEEL: f32` — the zoom factor of one pixel of wheel travel: a notch of an ordinary wheel is about 60 pixels, which makes a fifth.
- `pub const SHARP_SCALE: f32` — pixels of the picture drawn for each unit of the diagram when the viewer draws it again, so that it stays sharp when enlarged.
- `const MARGIN: f32` — left free around a diagram that is fitted into the stage.
- `const KEEP: f32` — how much of a diagram stays in view however far it is dragged.
- `pub fn fit(image: (f32, f32), stage: (f32, f32)) -> Camera` — the whole diagram in view, and never larger than its natural size: a small diagram is not blown up.
- `pub fn actual(image: (f32, f32), stage: (f32, f32)) -> Camera` — the diagram at its natural size. What does not fit starts at the top left corner, where a diagram is read from.
- `fn placed(image: (f32, f32), stage: (f32, f32), zoom: f32) -> Camera` — the diagram at `zoom`: centred along an axis it fits in, and from its start along one it does not.
- `pub fn zoom_by(camera: &mut Camera, sx: f32, sy: f32, factor: f32, stage: (f32, f32))` — zoom by `factor` within the viewer's limits, keeping the point of the diagram under `(sx, sy)` in place.
- `pub fn clamp(camera: &mut Camera, image: (f32, f32), stage: (f32, f32))` — bring a diagram that was moved out of the stage back, so that a part of it is always in view to take hold of.
- `pub fn frame(camera: &Camera, image: (f32, f32), stage: (f32, f32)) -> [f32; 4]` — the top left corner of the diagram in the stage, and its drawn size.
- `pub fn percent(camera: &Camera) -> u32` — the zoom as the viewer states it.
- `pub fn png_size(bytes: &[u8]) -> Option<(u32, u32)>` — width and height of a PNG in pixels, read from its header. `None` for anything that is not a PNG with a size.

## Depends on

- [[Graph Module (codewiki-app)]] — uses `Camera`, `to_screen`, `to_world`

## Used by

- [[codewiki-app]] — from `diagram_viewer_view`, `move_diagram`, `open_diagram`, `sharpen_diagram`

## See also

- [[_index]]
- [[codewiki-app]]
