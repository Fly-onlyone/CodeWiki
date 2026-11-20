//! Where an enlarged diagram sits in the diagram viewer.
//!
//! A note narrows a wide diagram to its column, which leaves the labels too
//! small to read. A click on the diagram opens it in the viewer, over the
//! window, where the wheel zooms and a drag moves it.
//!
//! The viewer looks at the diagram through a [`Camera`], as the graph view
//! looks at the graph: the world is the diagram, one unit per pixel of its
//! natural size, with the origin at its top left corner. All sizes here are
//! in design units. No view types, so it is unit-tested.

use crate::graph::Camera;

/// The smallest and the largest the viewer draws a diagram, as a share of
/// its natural size. Past four times, the picture is only blurred.
pub const MIN_ZOOM: f32 = 0.1;
pub const MAX_ZOOM: f32 = 4.0;
/// How much one press of a zoom button, or of `+` and `-`, changes the zoom.
pub const STEP: f32 = 1.25;
/// The zoom factor of one pixel of wheel travel: a notch of an ordinary
/// wheel is about 60 pixels, which makes a fifth.
pub const WHEEL: f32 = 1.003;
/// Pixels of the picture drawn for each unit of the diagram when the viewer
/// draws it again, so that it stays sharp when enlarged.
pub const SHARP_SCALE: f32 = 2.0;
/// Left free around a diagram that is fitted into the stage.
const MARGIN: f32 = 24.0;
/// How much of a diagram stays in view however far it is dragged.
const KEEP: f32 = 64.0;

/// The whole diagram in view, and never larger than its natural size: a
/// small diagram is not blown up.
pub fn fit(image: (f32, f32), stage: (f32, f32)) -> Camera {
    let across = (stage.0 - 2.0 * MARGIN).max(1.0) / image.0.max(1.0);
    let down = (stage.1 - 2.0 * MARGIN).max(1.0) / image.1.max(1.0);
    placed(image, stage, across.min(down).min(1.0))
}

/// The diagram at its natural size. What does not fit starts at the top left
/// corner, where a diagram is read from.
pub fn actual(image: (f32, f32), stage: (f32, f32)) -> Camera {
    placed(image, stage, 1.0)
}

/// The diagram at `zoom`: centred along an axis it fits in, and from its
/// start along one it does not.
fn placed(image: (f32, f32), stage: (f32, f32), zoom: f32) -> Camera {
    let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    let centre = |image: f32, stage: f32| {
        if image * zoom + 2.0 * MARGIN <= stage {
            image / 2.0
        } else {
            (stage / 2.0 - MARGIN) / zoom
        }
    };
    Camera {
        zoom,
        cx: centre(image.0, stage.0),
        cy: centre(image.1, stage.1),
    }
}

/// Zoom by `factor` within the viewer's limits, keeping the point of the
/// diagram under `(sx, sy)` in place.
pub fn zoom_by(camera: &mut Camera, sx: f32, sy: f32, factor: f32, stage: (f32, f32)) {
    // `Camera::zoom_at` with this view's limits in place of the graph's.
    let (x, y) = camera.to_world(sx, sy, stage.0, stage.1);
    camera.zoom = (camera.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    camera.cx = x - (sx - stage.0 / 2.0) / camera.zoom;
    camera.cy = y - (sy - stage.1 / 2.0) / camera.zoom;
}

/// Bring a diagram that was moved out of the stage back, so that a part of
/// it is always in view to take hold of.
pub fn clamp(camera: &mut Camera, image: (f32, f32), stage: (f32, f32)) {
    let hold = |centre: f32, image: f32, stage: f32| {
        let keep = KEEP.min(image * camera.zoom);
        let low = (keep - stage / 2.0) / camera.zoom;
        let high = (stage / 2.0 - keep) / camera.zoom + image;
        // A stage narrower than what is to stay in view holds nothing.
        if low <= high {
            centre.clamp(low, high)
        } else {
            centre
        }
    };
    camera.cx = hold(camera.cx, image.0, stage.0);
    camera.cy = hold(camera.cy, image.1, stage.1);
}

/// The top left corner of the diagram in the stage, and its drawn size.
pub fn frame(camera: &Camera, image: (f32, f32), stage: (f32, f32)) -> [f32; 4] {
    let (x, y) = camera.to_screen(0.0, 0.0, stage.0, stage.1);
    [x, y, image.0 * camera.zoom, image.1 * camera.zoom]
}

/// The zoom as the viewer states it.
pub fn percent(camera: &Camera) -> u32 {
    (camera.zoom * 100.0).round() as u32
}

/// Width and height of a PNG in pixels, read from its header. `None` for
/// anything that is not a PNG with a size.
pub fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    if bytes.get(..8)? != SIGNATURE || bytes.get(12..16)? != b"IHDR" {
        return None;
    }
    let field = |at: usize| Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    let (width, height) = (field(16)?, field(20)?);
    (width > 0 && height > 0).then_some((width, height))
}

#[cfg(test)]
mod tests {
    use super::{
        Camera, MARGIN, MAX_ZOOM, MIN_ZOOM, actual, clamp, fit, frame, percent, png_size, zoom_by,
    };

    const STAGE: (f32, f32) = (1000.0, 600.0);
    /// As wide as the startup flow of the app, which a note shows at 28%.
    const WIDE: (f32, f32) = (2400.0, 800.0);
    const SMALL: (f32, f32) = (300.0, 200.0);

    #[test]
    fn a_wide_diagram_is_fitted_whole_into_the_stage() {
        let camera = fit(WIDE, STAGE);
        let [x, y, width, height] = frame(&camera, WIDE, STAGE);
        assert!((x - MARGIN).abs() < 0.01, "{x}");
        assert!((x + width - (STAGE.0 - MARGIN)).abs() < 0.01);
        assert!(
            y >= MARGIN && y + height <= STAGE.1 - MARGIN,
            "{y} {height}"
        );
        // Centred down the stage, which it does not fill.
        assert!((y + height / 2.0 - STAGE.1 / 2.0).abs() < 0.01);
    }

    #[test]
    fn a_small_diagram_is_not_enlarged_to_fit() {
        let camera = fit(SMALL, STAGE);
        assert_eq!(camera.zoom, 1.0);
        let [x, y, width, height] = frame(&camera, SMALL, STAGE);
        assert_eq!([width, height], [SMALL.0, SMALL.1]);
        assert_eq!([x, y], [350.0, 200.0]);
    }

    #[test]
    fn the_natural_size_starts_a_wide_diagram_at_its_left() {
        let camera = actual(WIDE, STAGE);
        assert_eq!(percent(&camera), 100);
        let [x, y, width, _] = frame(&camera, WIDE, STAGE);
        assert!((x - MARGIN).abs() < 0.01, "{x}");
        assert!((y - MARGIN).abs() < 0.01, "{y}");
        assert_eq!(width, WIDE.0);
    }

    #[test]
    fn zooming_keeps_the_point_under_the_pointer() {
        let mut camera = fit(WIDE, STAGE);
        let before = camera.to_world(700.0, 250.0, STAGE.0, STAGE.1);
        zoom_by(&mut camera, 700.0, 250.0, 1.8, STAGE);
        let after = camera.to_world(700.0, 250.0, STAGE.0, STAGE.1);
        assert!((before.0 - after.0).abs() < 0.01 && (before.1 - after.1).abs() < 0.01);
    }

    #[test]
    fn the_zoom_stays_within_its_limits() {
        let mut camera = fit(WIDE, STAGE);
        zoom_by(&mut camera, 500.0, 300.0, 1000.0, STAGE);
        assert_eq!(camera.zoom, MAX_ZOOM);
        zoom_by(&mut camera, 500.0, 300.0, 0.000_01, STAGE);
        assert_eq!(camera.zoom, MIN_ZOOM);
        assert_eq!(percent(&camera), 10);
    }

    #[test]
    fn a_diagram_cannot_be_dragged_out_of_view() {
        for (dx, dy) in [(1e6, 0.0), (-1e6, 0.0), (0.0, 1e6), (0.0, -1e6)] {
            let mut camera = actual(WIDE, STAGE);
            camera.pan(dx, dy);
            clamp(&mut camera, WIDE, STAGE);
            let [x, y, width, height] = frame(&camera, WIDE, STAGE);
            assert!(x < STAGE.0 - 1.0 && x + width > 1.0, "{x} {width}");
            assert!(y < STAGE.1 - 1.0 && y + height > 1.0, "{y} {height}");
        }
        // One that is in view is left where it is.
        let mut camera = fit(WIDE, STAGE);
        let placed = camera;
        clamp(&mut camera, WIDE, STAGE);
        assert_eq!(camera, placed);
        // A stage too small to hold anything changes nothing and does not
        // panic.
        let mut camera = Camera {
            zoom: 1.0,
            cx: 5.0,
            cy: 5.0,
        };
        clamp(&mut camera, (100.0, 100.0), (10.0, 10.0));
        assert_eq!((camera.cx, camera.cy), (5.0, 5.0));
    }

    #[test]
    fn reads_the_size_of_a_png_and_of_nothing_else() {
        let png = cw_mermaid::render_png("flowchart LR\n    A --> B\n", cw_mermaid::Palette::Light)
            .unwrap();
        let (width, height) = png_size(&png).unwrap();
        assert!(width > 20 && height > 20, "{width} x {height}");
        assert_eq!(png_size(&png[..20]), None);
        assert_eq!(png_size(b"GIF89a, and then some more bytes"), None);
        assert_eq!(png_size(&[]), None);
        let mut empty = png.clone();
        empty[16..20].fill(0);
        assert_eq!(png_size(&empty), None);
    }
}
