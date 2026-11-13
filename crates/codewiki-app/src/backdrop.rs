//! The drifting backdrop: soft color under the glass.
//!
//! A few large soft shapes in the theme's colors sit behind every surface
//! and drift slowly. They are what the glass has to show when the wallpaper
//! behind the window is plain.
//!
//! gpui-kit has no radial gradient, and its box shadow spreads a blur this
//! wide so thin that the color all but vanishes. A shape is therefore a stack
//! of faint discs, each smaller than the last, which adds up to a color that
//! is strongest in the middle and fades to nothing at the rim.
//!
//! [`Shell`] is the window's outermost view. It draws the shapes and, on top,
//! the app and the generated page, each as a cached view. Only the shell is
//! redrawn when the shapes move; the app and the page keep their last frames
//! until they change themselves, and neither is redrawn when the other is.
//!
//! The shapes stand still when the window is inactive and under reduced
//! motion, so an idle or backgrounded window costs nothing. They also stand
//! still when Direct3D runs on the processor (a virtual machine, a remote
//! session, a broken graphics driver): there every frame of the drift is
//! drawn by the processor, and in a clean-Windows test an idle window kept
//! both cores of the machine busy and starved everything else.

use std::f32::consts::TAU;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, Context, Div, Entity, Hsla, IntoElement, ParentElement as _, Render, StyleRefinement,
    Styled as _, Subscription, Window, div, px,
};

use crate::doc_view::{DocPage, PageSlot};
use crate::glass;

/// Seconds for the shapes to come back to where they started.
const PERIOD: f32 = 90.0;
/// Time between two positions. About 24 a second: the drift is too slow to
/// need more.
const TICK: Duration = Duration::from_millis(42);
/// The longest step one frame may take, so that a stalled frame does not show
/// as a jump.
const MAX_STEP: f32 = 0.25;

/// How strongly a shape colors the backdrop at its middle. `glass` allows for
/// this much when it works out how opaque the chrome must be.
pub const STRENGTH: f32 = 0.26;
// A tint, not a wash: `glass` widens what it expects behind the chrome by
// this much, and a strong backdrop would force the chrome nearly opaque.
const _: () = assert!(STRENGTH <= 0.3);

/// Discs in one shape. With 18 the steps between them showed as faint rings;
/// at 48 each step is smaller than one level of an 8-bit display.
const RINGS: usize = 48;

/// Radius of disc `index` of a shape, as a share of the shape's radius. The
/// first is the whole shape; they shrink quickly at first and slowly near the
/// middle, so the color rises gently from the rim.
fn ring_radius(index: usize) -> f32 {
    1.0 - (index as f32 / RINGS as f32).sqrt()
}

/// The opacity of each disc such that all of them stacked reach `total`.
fn ring_alpha(total: f32) -> f32 {
    1.0 - (1.0 - total).powf(1.0 / RINGS as f32)
}

/// How GPUI names an adapter from Microsoft's vendor ID, 0x1414.
const MICROSOFT_VENDOR: &str = "(ID: 0x1414)";

/// Whether an adapter draws on the processor, from what GPUI reports of it.
///
/// DXGI's software flag is not enough. Without a graphics driver (a virtual
/// machine without a graphics card, some remote sessions) Windows renders
/// with the "Microsoft Basic Render Driver", which is software but does not
/// carry the flag. Microsoft's own adapters are known by their vendor ID.
fn draws_on_processor(software_flag: bool, device: &str, vendor: &str) -> bool {
    software_flag
        || vendor.contains(MICROSOFT_VENDOR)
        || device.starts_with("Microsoft Basic Render")
}

/// One shape: where it rests, how far it wanders, and how fast.
struct Blob {
    /// Resting place, as a share of the window's width and height.
    at: (f32, f32),
    /// Diameter, as a share of the window's shorter side.
    size: f32,
    /// How far it wanders from its resting place, in the same unit as `size`.
    reach: f32,
    /// Whole turns per period, across and down. Whole numbers close the loop.
    turns: (f32, f32),
    /// Where in its turn it starts, from 0 to 1.
    offset: f32,
    /// Share of `STRENGTH` this shape gets.
    weight: f32,
}

/// The shapes sit behind the chrome — the sidebar, the top band, the details
/// column — and stay off the middle, where the page is.
const BLOBS: [Blob; 4] = [
    Blob {
        at: (0.06, 0.20),
        size: 0.55,
        reach: 0.07,
        turns: (1.0, 2.0),
        offset: 0.0,
        weight: 1.0,
    },
    Blob {
        at: (0.10, 0.92),
        size: 0.45,
        reach: 0.06,
        turns: (2.0, 1.0),
        offset: 0.35,
        weight: 0.7,
    },
    Blob {
        at: (0.72, 0.0),
        size: 0.40,
        reach: 0.08,
        turns: (1.0, 3.0),
        offset: 0.6,
        weight: 0.7,
    },
    Blob {
        at: (0.97, 0.78),
        size: 0.50,
        reach: 0.06,
        turns: (2.0, 3.0),
        offset: 0.8,
        weight: 0.85,
    },
];

impl Blob {
    /// The shape's displacement from its resting place at `phase` (0 to 1),
    /// in the same unit as `size`.
    fn wander(&self, phase: f32) -> (f32, f32) {
        let angle = |turns: f32| TAU * (phase * turns + self.offset);
        (
            self.reach * angle(self.turns.0).sin(),
            self.reach * angle(self.turns.1).cos(),
        )
    }
}

/// `phase` moved on by `seconds`, wrapped into 0 to 1.
fn advance(phase: f32, seconds: f32) -> f32 {
    (phase + seconds.clamp(0.0, MAX_STEP) / PERIOD).rem_euclid(1.0)
}

/// The window's outermost view: the drifting shapes, and `content` over them.
pub struct Shell<V: Render> {
    content: Entity<V>,
    /// The generated page, and where the app wants it.
    page: Entity<DocPage>,
    slot: Rc<PageSlot>,
    /// Opaque color painted under everything in place of the real backdrop.
    debug_backdrop: Option<Hsla>,
    /// Drift even while the window is inactive, so that a capture taken
    /// without giving the window focus can see it move.
    always_drift: bool,
    /// The graphics adapter is a software one, so the shapes stand still
    /// unless `always_drift` says otherwise.
    software: bool,
    phase: f32,
    /// When the shapes last moved; `None` while they stand still.
    moved_at: Option<Instant>,
    tick_pending: bool,
    _activation: Subscription,
}

impl<V: Render> Shell<V> {
    pub fn new(
        content: Entity<V>,
        page: Entity<DocPage>,
        slot: Rc<PageSlot>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        slot.read_by(cx.entity_id());
        // Start and stop drifting as the window gains and loses focus.
        let activation = cx.observe_window_activation(window, |_, _, cx| cx.notify());
        let specs = window.gpu_specs();
        let software = specs.as_ref().is_some_and(|gpu| {
            draws_on_processor(gpu.is_software_emulated, &gpu.device_name, &gpu.driver_name)
        });
        match &specs {
            Some(gpu) => log::info!(
                "graphics adapter: {} ({}, {}), software: {software}",
                gpu.device_name,
                gpu.driver_name,
                gpu.driver_info,
            ),
            None => log::info!("graphics adapter: not reported"),
        }
        Self {
            content,
            page,
            slot,
            debug_backdrop: glass::debug_backdrop(),
            always_drift: std::env::var_os("CODEWIKI_BACKDROP_DRIFT").is_some(),
            software,
            phase: 0.0,
            moved_at: None,
            tick_pending: false,
            _activation: activation,
        }
    }

    /// Move the shapes on by the time since the last frame, and ask for the
    /// next frame. Does nothing while the shapes should stand still.
    fn drift(&mut self, window: &Window, cx: &mut Context<Self>) {
        let moving = !cx.reduce_motion()
            && (self.always_drift || (!self.software && window.is_window_active()));
        if !moving {
            self.moved_at = None;
            return;
        }
        let now = Instant::now();
        if let Some(before) = self.moved_at {
            self.phase = advance(self.phase, (now - before).as_secs_f32());
        }
        self.moved_at = Some(now);
        if !self.tick_pending {
            self.tick_pending = true;
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(TICK).await;
                this.update(cx, |this, cx| {
                    this.tick_pending = false;
                    cx.notify();
                })
                .ok();
            })
            .detach();
        }
    }

    fn shapes(&self, window: &Window, cx: &App) -> Div {
        let theme = cx.theme();
        let colors = [theme.primary, theme.info, theme.magenta, theme.primary];
        let viewport = window.viewport_size();
        let (width, height) = (viewport.width / px(1.), viewport.height / px(1.));
        let unit = width.min(height);
        let mut layer = div().absolute().inset_0().overflow_hidden();
        for (blob, color) in BLOBS.iter().zip(colors) {
            let (dx, dy) = blob.wander(self.phase);
            let centre = (
                blob.at.0 * width + dx * unit,
                blob.at.1 * height + dy * unit,
            );
            let radius = blob.size * unit / 2.0;
            let tint = color.opacity(ring_alpha(STRENGTH * blob.weight));
            for ring in 0..RINGS {
                let r = radius * ring_radius(ring);
                layer = layer.child(
                    div()
                        .absolute()
                        .left(px(centre.0 - r))
                        .top(px(centre.1 - r))
                        .size(px(r * 2.0))
                        .rounded_full()
                        .bg(tint),
                );
            }
        }
        layer
    }
}

impl<V: Render> Render for Shell<V> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.drift(window, cx);
        div()
            .size_full()
            .relative()
            // Painted first, so under everything.
            .when_some(self.debug_backdrop, |shell, color| {
                shell.child(div().absolute().inset_0().bg(color))
            })
            .child(self.shapes(window, cx))
            .child(
                self.content
                    .clone()
                    .cached(StyleRefinement::default().size_full()),
            )
            // The page, in the place the app left empty for it.
            .when_some(self.slot.bounds(), |shell, place| {
                shell.child(
                    div()
                        .absolute()
                        .left(place.origin.x)
                        .top(place.origin.y)
                        .w(place.size.width)
                        .h(place.size.height)
                        .child(
                            self.page
                                .clone()
                                .cached(StyleRefinement::default().size_full()),
                        ),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_software_adapter_is_known_with_or_without_the_flag() {
        // What a Hyper-V guest without a graphics card reported: no flag.
        assert!(draws_on_processor(
            false,
            "Microsoft Basic Render Driver",
            "Unknown Vendor (ID: 0x1414)"
        ));
        assert!(draws_on_processor(
            true,
            "Some Adapter",
            "Unknown Vendor (ID: 0x1234)"
        ));
        // A real card, as the development machine reported it.
        assert!(!draws_on_processor(
            false,
            "NVIDIA GeForce RTX 4070",
            "NVIDIA Corporation"
        ));
        assert!(!draws_on_processor(
            false,
            "Intel(R) UHD Graphics",
            "Intel Corporation"
        ));
    }

    #[test]
    fn the_drift_closes_its_loop() {
        for blob in &BLOBS {
            let (x0, y0) = blob.wander(0.0);
            let (x1, y1) = blob.wander(1.0);
            assert!((x0 - x1).abs() < 1e-4 && (y0 - y1).abs() < 1e-4);
        }
    }

    #[test]
    fn every_shape_moves_and_stays_within_its_reach() {
        for blob in &BLOBS {
            let start = blob.wander(0.0);
            let mut moved = false;
            for step in 0..=200 {
                let (x, y) = blob.wander(step as f32 / 200.0);
                assert!(x.abs() <= blob.reach + 1e-4, "x {x} beyond {}", blob.reach);
                assert!(y.abs() <= blob.reach + 1e-4, "y {y} beyond {}", blob.reach);
                moved |= (x - start.0).abs() > blob.reach * 0.5;
            }
            assert!(moved, "a shape never left its starting point");
        }
    }

    #[test]
    fn shapes_rest_behind_the_chrome_not_the_page() {
        // The page is the middle of the window: past the sidebar on the left,
        // short of the details column on the right, below the top band.
        for blob in &BLOBS {
            let (x, y) = blob.at;
            let behind_chrome = x < 0.18 || x > 0.84 || y < 0.08;
            assert!(
                behind_chrome,
                "a shape rests at ({x}, {y}), behind the page"
            );
        }
    }

    #[test]
    fn advancing_wraps_and_never_jumps() {
        assert_eq!(advance(0.0, 0.0), 0.0);
        let step = advance(0.0, 0.042);
        assert!(step > 0.0 && step < 0.001);
        // A frame that took far too long moves the shapes one capped step.
        assert_eq!(advance(0.0, 60.0), advance(0.0, MAX_STEP));
        // The end of the loop is its beginning.
        let wrapped = advance(0.9999, MAX_STEP);
        assert!((0.0..0.01).contains(&wrapped), "{wrapped}");
        // Time never runs backwards.
        assert_eq!(advance(0.5, -3.0), 0.5);
    }

    #[test]
    fn the_discs_of_a_shape_shrink_to_its_middle() {
        assert_eq!(ring_radius(0), 1.0);
        let radii: Vec<f32> = (0..RINGS).map(ring_radius).collect();
        assert!(radii.windows(2).all(|pair| pair[0] > pair[1]), "{radii:?}");
        assert!(radii.iter().all(|&r| r > 0.0));
        // The outer discs are far apart and the inner ones close: a soft rim.
        assert!(radii[0] - radii[1] > radii[RINGS - 2] - radii[RINGS - 1]);
    }

    #[test]
    fn the_discs_stacked_reach_the_strength_asked_for() {
        for total in [0.1, STRENGTH, 0.5] {
            let each = ring_alpha(total);
            let stacked = 1.0 - (1.0 - each).powi(RINGS as i32);
            assert!((stacked - total).abs() < 1e-4, "{stacked} for {total}");
            assert!(each < total);
        }
    }

    #[test]
    fn no_shape_is_stronger_than_the_strength_glass_allows_for() {
        assert!(
            BLOBS
                .iter()
                .all(|blob| blob.weight > 0.0 && blob.weight <= 1.0)
        );
    }
}
