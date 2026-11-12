//! The ripple a control answers a press with.
//!
//! Follows MUI's `ButtonBase/TouchRipple.js`: a disc grows from the point of
//! the press to cover the control, and fades once the press ends.
//!
//! GPUI clips to rectangles only, so a disc inside a control with rounded
//! corners would show beyond them. Each ripple is therefore painted as the
//! shape it should have: the disc intersected with the control's rounded
//! rectangle. Both are convex, so the intersection is one polygon.
//!
//! Lengths are in pixels and points are relative to the control's top-left
//! corner.

use std::f32::consts::{FRAC_PI_2, TAU};
use std::time::{Duration, Instant};

use gpui_kit::{Bounds, Hsla, PathBuilder, Pixels, Window, fill, point, px};

use super::motion::scaled;
use super::tokens::{curve, easing};

/// `TouchRipple.js`, `DURATION`: the time a ripple takes to grow, and to fade.
pub const DURATION: Duration = Duration::from_millis(550);
/// `enterKeyframe`: the opacity a ripple starts and ends its growth at.
const ENTER_OPACITY: (f32, f32) = (0.1, 0.3);
/// Sides of the polygon that stands in for the disc.
const DISC_SIDES: usize = 64;

pub type Point = (f32, f32);

/// Where a ripple starts. `TouchRipple.js`, `start`: the middle of the
/// control for a centered ripple or a press without a position, else the
/// point pressed.
pub fn origin(width: f32, height: f32, press: Option<Point>, centered: bool) -> Point {
    match press {
        Some((x, y)) if !centered => (x.round(), y.round()),
        _ => ((width / 2.0).round(), (height / 2.0).round()),
    }
}

/// The diameter a ripple grows to. `TouchRipple.js`, `rippleSize`: wide
/// enough to reach the corner farthest from `origin`; a centered ripple is
/// smaller, and leaves the corners of a square control clear.
pub fn size(width: f32, height: f32, origin: Point, centered: bool) -> f32 {
    if centered {
        ((2.0 * width * width + height * height) / 3.0).sqrt()
    } else {
        let across = (width - origin.0).abs().max(origin.0) * 2.0 + 2.0;
        let down = (height - origin.1).abs().max(origin.1) * 2.0 + 2.0;
        across.hypot(down)
    }
}

/// One ripple: when the press began and, once it has, when it ended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wave {
    /// The point pressed, in window coordinates; `None` for a keyboard press.
    pub press: Option<Point>,
    pub started: Instant,
    pub released: Option<Instant>,
}

impl Wave {
    pub fn new(press: Option<Point>) -> Self {
        Self {
            press,
            started: Instant::now(),
            released: None,
        }
    }

    pub fn release(&mut self) {
        self.released.get_or_insert_with(Instant::now);
    }

    /// How far the disc has grown (0 to 1) and how opaque it is, at `now`.
    pub fn frame(&self, now: Instant) -> (f32, f32) {
        let fading = self.released.map(|at| now.saturating_duration_since(at));
        frame(now.saturating_duration_since(self.started), fading)
    }

    /// Whether the ripple still changes from frame to frame. One held down
    /// at full size does not.
    pub fn moving(&self, now: Instant) -> bool {
        self.released.is_some() || now.saturating_duration_since(self.started) < scaled(DURATION)
    }

    /// Whether the ripple has faded out and can be dropped.
    pub fn finished(&self, now: Instant) -> bool {
        self.released
            .is_some_and(|at| now.saturating_duration_since(at) >= scaled(DURATION))
    }
}

/// Growth and opacity of a ripple that began `growing` ago and whose press
/// ended `fading` ago. Growth and fade run side by side, as in MUI, where a
/// quick click starts the fade while the disc is still growing.
fn frame(growing: Duration, fading: Option<Duration>) -> (f32, f32) {
    let ease = easing(curve::EASE_IN_OUT);
    let share = |elapsed: Duration| {
        (elapsed.as_secs_f32() / scaled(DURATION).as_secs_f32()).clamp(0.0, 1.0)
    };
    let grown = ease(share(growing));
    let visible = ENTER_OPACITY.0 + (ENTER_OPACITY.1 - ENTER_OPACITY.0) * grown;
    let left = fading.map_or(1.0, |elapsed| 1.0 - ease(share(elapsed)));
    (grown, visible * left)
}

/// The outline of a `width` by `height` rectangle with corners of `radius`,
/// as a convex polygon.
pub fn rounded_rect(width: f32, height: f32, radius: f32) -> Vec<Point> {
    let radius = radius.clamp(0.0, width.min(height) / 2.0);
    // Enough sides that a corner stays round to the eye.
    let steps = if radius < 0.5 {
        0
    } else {
        ((radius / 1.5).ceil() as usize).clamp(4, 16)
    };
    // Corner centres in the order of increasing angle, with the y axis down:
    // bottom-right, bottom-left, top-left, top-right.
    let centres = [
        (width - radius, height - radius),
        (radius, height - radius),
        (radius, radius),
        (width - radius, radius),
    ];
    let mut outline = Vec::with_capacity(4 * (steps + 1));
    for (corner, (cx, cy)) in centres.into_iter().enumerate() {
        let start = corner as f32 * FRAC_PI_2;
        for step in 0..=steps {
            let angle = start + FRAC_PI_2 * step as f32 / steps.max(1) as f32;
            outline.push((cx + radius * angle.cos(), cy + radius * angle.sin()));
        }
    }
    outline
}

/// A disc as a polygon, wound the same way as [`rounded_rect`].
pub fn disc(centre: Point, radius: f32) -> Vec<Point> {
    (0..DISC_SIDES)
        .map(|side| {
            let angle = TAU * side as f32 / DISC_SIDES as f32;
            (
                centre.0 + radius * angle.cos(),
                centre.1 + radius * angle.sin(),
            )
        })
        .collect()
}

/// The part of `subject` inside the convex polygon `bounds` (Sutherland and
/// Hodgman). Both must be wound as [`rounded_rect`] is.
pub fn clip(subject: &[Point], bounds: &[Point]) -> Vec<Point> {
    let mut kept = subject.to_vec();
    for (index, &from) in bounds.iter().enumerate() {
        let to = bounds[(index + 1) % bounds.len()];
        // Which side of the edge a point is on; positive is inside.
        let side = |p: Point| (to.0 - from.0) * (p.1 - from.1) - (to.1 - from.1) * (p.0 - from.0);
        let input = std::mem::take(&mut kept);
        for (at, &current) in input.iter().enumerate() {
            let previous = input[(at + input.len() - 1) % input.len()];
            let (now, before) = (side(current), side(previous));
            if (now >= 0.0) != (before >= 0.0) {
                let cut = before / (before - now);
                kept.push((
                    previous.0 + (current.0 - previous.0) * cut,
                    previous.1 + (current.1 - previous.1) * cut,
                ));
            }
            if now >= 0.0 {
                kept.push(current);
            }
        }
        if kept.is_empty() {
            break;
        }
    }
    kept
}

/// Whether a disc covers the whole `width` by `height` rectangle.
pub fn covers(centre: Point, radius: f32, width: f32, height: f32) -> bool {
    [(0.0, 0.0), (width, 0.0), (0.0, height), (width, height)]
        .into_iter()
        .all(|(x, y)| (x - centre.0).hypot(y - centre.1) <= radius)
}

/// Paint `waves` over a control at `bounds` whose corners have `radius`.
/// Call while painting. `centered` ripples start from the middle, as on an
/// icon button.
pub fn paint(
    bounds: Bounds<Pixels>,
    radius: f32,
    waves: &[Wave],
    centered: bool,
    color: Hsla,
    window: &mut Window,
) {
    let (left, top) = (bounds.origin.x / px(1.), bounds.origin.y / px(1.));
    let (width, height) = (bounds.size.width / px(1.), bounds.size.height / px(1.));
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let now = Instant::now();
    let outline = rounded_rect(width, height, radius);
    for wave in waves {
        let (grown, opacity) = wave.frame(now);
        if opacity <= 0.0 || grown <= 0.0 {
            continue;
        }
        let press = wave.press.map(|(x, y)| (x - left, y - top));
        let centre = origin(width, height, press, centered);
        let reach = size(width, height, centre, centered) / 2.0 * grown;
        let tint = color.opacity(color.a * opacity);
        if covers(centre, reach, width, height) {
            // The disc has passed every corner: the control's own shape.
            window.paint_quad(fill(bounds, tint).corner_radii(px(radius)));
            continue;
        }
        let shape = clip(&disc(centre, reach), &outline);
        if shape.len() < 3 {
            continue;
        }
        let points: Vec<_> = shape
            .into_iter()
            .map(|(x, y)| point(px(left + x), px(top + y)))
            .collect();
        let mut builder = PathBuilder::fill();
        builder.add_polygon(&points, true);
        if let Ok(path) = builder.build() {
            window.paint_path(path, tint);
        }
    }
}

/// The tint a pressed control shows in place of a ripple when motion is
/// reduced: the ripple's resting opacity over the whole control.
pub fn paint_pressed(bounds: Bounds<Pixels>, radius: f32, color: Hsla, window: &mut Window) {
    let tint = color.opacity(color.a * ENTER_OPACITY.1);
    window.paint_quad(fill(bounds, tint).corner_radii(px(radius)));
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: fn(u64) -> Duration = Duration::from_millis;

    /// Twice the area of a polygon; positive when wound as `rounded_rect`.
    fn doubled_area(points: &[Point]) -> f32 {
        (0..points.len())
            .map(|i| {
                let (a, b) = (points[i], points[(i + 1) % points.len()]);
                a.0 * b.1 - b.0 * a.1
            })
            .sum()
    }

    /// Whether a point is inside a rounded rectangle, by its definition.
    fn in_rounded_rect(p: Point, width: f32, height: f32, radius: f32) -> bool {
        let slack = 0.05;
        if p.0 < -slack || p.1 < -slack || p.0 > width + slack || p.1 > height + slack {
            return false;
        }
        let nearest = |v: f32, size: f32| v.clamp(radius, size - radius);
        let (cx, cy) = (nearest(p.0, width), nearest(p.1, height));
        (p.0 - cx).hypot(p.1 - cy) <= radius + slack
    }

    #[test]
    fn a_ripple_starts_where_it_was_pressed() {
        assert_eq!(origin(100.0, 40.0, Some((12.4, 30.6)), false), (12.0, 31.0));
        // A keyboard press, or a centered ripple, starts in the middle.
        assert_eq!(origin(100.0, 40.0, None, false), (50.0, 20.0));
        assert_eq!(origin(100.0, 40.0, Some((1.0, 1.0)), true), (50.0, 20.0));
    }

    #[test]
    fn a_ripple_reaches_the_farthest_corner() {
        // Pressed at the left edge of a 100 by 40 control: MUI's own sum.
        let diameter = size(100.0, 40.0, (0.0, 20.0), false);
        assert!((diameter - 202.0f32.hypot(42.0)).abs() < 1e-3);
        for press in [(0.0, 0.0), (100.0, 40.0), (37.0, 5.0), (50.0, 20.0)] {
            let reach = size(100.0, 40.0, press, false) / 2.0;
            assert!(covers(press, reach, 100.0, 40.0), "{press:?}");
        }
    }

    #[test]
    fn a_centered_ripple_is_the_size_mui_gives_it() {
        // √((2·40² + 40²) / 3) = 40: a 40 px icon button gets a 40 px disc.
        assert!((size(40.0, 40.0, (20.0, 20.0), true) - 40.0).abs() < 1e-4);
        assert!(!covers((20.0, 20.0), 20.0, 40.0, 40.0));
    }

    #[test]
    fn a_ripple_grows_brightens_and_then_fades() {
        assert_eq!(frame(MS(0), None).0, 0.0);
        assert!((frame(MS(0), None).1 - 0.1).abs() < 1e-3);
        let (grown, opacity) = frame(MS(550), None);
        assert!((grown - 1.0).abs() < 1e-3 && (opacity - 0.3).abs() < 1e-3);
        // Held down, it stays.
        assert_eq!(frame(MS(5000), None), frame(MS(550), None));
        // Released, it fades over the same time.
        let half = frame(MS(825), Some(MS(275))).1;
        assert!(half > 0.0 && half < 0.3);
        assert!(frame(MS(1100), Some(MS(550))).1.abs() < 1e-3);
        // A quick click fades while it is still growing.
        let (grown, opacity) = frame(MS(200), Some(MS(150)));
        assert!(grown > 0.0 && grown < 1.0 && opacity > 0.0);
    }

    #[test]
    fn a_wave_is_dropped_once_it_has_faded() {
        let mut wave = Wave::new(Some((3.0, 4.0)));
        let later = wave.started + Duration::from_secs(60);
        assert!(!wave.finished(later), "a held press never finishes");
        assert!(wave.moving(wave.started + MS(10)));
        assert!(!wave.moving(later), "held at full size, it is still");
        wave.release();
        assert!(wave.moving(later), "fading, it moves again");
        let released = wave.released.expect("released");
        wave.release();
        assert_eq!(wave.released, Some(released), "the first release counts");
        assert!(!wave.finished(released + MS(100)));
        assert!(wave.finished(released + scaled(DURATION)));
    }

    #[test]
    fn a_rounded_rect_is_convex_and_stays_in_its_box() {
        for (width, height, radius) in [
            (100.0, 40.0, 8.0),
            (40.0, 40.0, 20.0),
            (64.0, 30.75, 4.0),
            (50.0, 20.0, 0.0),
            (30.0, 20.0, 99.0),
        ] {
            let outline = rounded_rect(width, height, radius);
            assert!(outline.len() >= 4);
            assert!(doubled_area(&outline) > 0.0);
            for (index, &a) in outline.iter().enumerate() {
                let b = outline[(index + 1) % outline.len()];
                let c = outline[(index + 2) % outline.len()];
                let turn = (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0);
                assert!(
                    turn >= -1e-3,
                    "a dent at {index} of {width}x{height} r{radius}"
                );
                assert!(a.0 >= -1e-3 && a.0 <= width + 1e-3);
                assert!(a.1 >= -1e-3 && a.1 <= height + 1e-3);
            }
        }
        // A square corner is one point, not an arc.
        assert_eq!(rounded_rect(50.0, 20.0, 0.0).len(), 4);
    }

    #[test]
    fn a_clipped_ripple_never_leaves_the_control() {
        let (width, height, radius) = (100.0, 40.0, 8.0);
        let outline = rounded_rect(width, height, radius);
        // Pressed in a corner, where a plain disc would show past the curve.
        for press in [(2.0, 2.0), (98.0, 38.0), (50.0, 20.0), (0.0, 20.0)] {
            for reach in [3.0, 10.0, 25.0, 60.0] {
                let shape = clip(&disc(press, reach), &outline);
                for &p in &shape {
                    assert!(
                        in_rounded_rect(p, width, height, radius),
                        "{p:?} outside for press {press:?} reach {reach}"
                    );
                    assert!((p.0 - press.0).hypot(p.1 - press.1) <= reach + 0.05);
                }
                assert!(doubled_area(&shape) > 0.0);
            }
        }
    }

    #[test]
    fn a_small_ripple_in_the_middle_is_left_whole() {
        let outline = rounded_rect(100.0, 40.0, 8.0);
        let whole = disc((50.0, 20.0), 10.0);
        let clipped = clip(&whole, &outline);
        assert_eq!(clipped.len(), whole.len());
        assert!((doubled_area(&clipped) - doubled_area(&whole)).abs() < 1e-2);
    }

    #[test]
    fn a_ripple_that_covers_the_control_takes_its_shape() {
        let outline = rounded_rect(100.0, 40.0, 8.0);
        let clipped = clip(&disc((50.0, 20.0), 500.0), &outline);
        let ratio = doubled_area(&clipped) / doubled_area(&outline);
        assert!((ratio - 1.0).abs() < 1e-3, "{ratio}");
        assert!(covers((50.0, 20.0), 500.0, 100.0, 40.0));
        assert!(!covers((50.0, 20.0), 30.0, 100.0, 40.0));
    }

    #[test]
    fn a_ripple_outside_the_control_is_nothing() {
        let outline = rounded_rect(100.0, 40.0, 8.0);
        assert!(clip(&disc((300.0, 300.0), 10.0), &outline).is_empty());
    }
}
