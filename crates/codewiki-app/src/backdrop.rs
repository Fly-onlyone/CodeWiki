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
//! Over the color lies a sky of notes and the links between them
//! (`constellation.rs`), which says what the app is about. It is drawn only
//! in the clear margins beside the reading column ([`Margins`]): there the
//! page's glass is at its thinnest and carries no text, so the sky can be
//! strong without making anything harder to read. None of it is behind the
//! chrome, whose glass `glass` fits to [`STRENGTH`] of the shapes alone, nor
//! behind the column, where moving light would pull the eye off the text.
//! Light runs along the links: now and then by itself, and often while a wiki
//! is being written ([`set_writing`]). What is near the pointer brightens.
//!
//! Settings → Appearance turns the whole backdrop off ([`set_shown`]): plain
//! Mica behind the glass, and a page as dense as its column.
//!
//! [`Shell`] is the window's outermost view. It draws the shapes and, on top,
//! the app and the generated page, each as a cached view. Only the shell is
//! redrawn when the shapes move; the app and the page keep their last frames
//! until they change themselves, and neither is redrawn when the other is.
//!
//! The title bar is the exception: the shell draws it itself, in every frame.
//! GPUI keeps the hit areas of a cached view but not its window control
//! areas (the minimize, maximize and close buttons and the part that drags
//! the window; `window_control_hitboxes` in gpui's `window.rs` is cleared for
//! each frame and filled only by a view that is drawn anew). While the title
//! bar was part of the cached app, the buttons and the bar did nothing in
//! every frame where only the shapes had moved, which with the drift on is
//! nearly every frame.
//!
//! The shell also takes Ctrl+wheel for `zoom`, ahead of everything under the
//! pointer, and draws the title bar at 100% whatever the zoom. Its shapes are
//! shares of the window and keep GPUI's own pixels.
//!
//! The shapes stand still when the window is inactive and under reduced
//! motion, so an idle or backgrounded window costs nothing; so do the sky, its
//! pulses and the pointer's light. They also stand
//! still when Direct3D runs on the processor (a virtual machine, a remote
//! session, a broken graphics driver): there every frame of the drift is
//! drawn by the processor, and in a clean-Windows test an idle window kept
//! both cores of the machine busy and starved everything else.

use std::cell::{Cell, RefCell};
use std::f32::consts::TAU;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Bounds, ContentMask, Context, DispatchPhase, Div, Entity, EntityId, Global,
    Hsla, IntoElement, ParentElement as _, PathBuilder, Pixels, Point, Render, ScrollEvent,
    StyleRefinement, Styled as _, Subscription, Window, canvas, div, point, px,
};

use crate::constellation::{self, CLUSTERS, Constellation, Pulse, Traffic};
use crate::doc_view::{self, DocPage, PageSlot};
use crate::{glass, prefs, win32, zoom};

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

/// How strongly each part of the sky colors what is behind the margins'
/// glass. The margins carry no text, so no contrast limit applies to these.
/// These and the pointer's light, the sizes and widths below, were chosen by
/// eye on 2026-11-24.
const LINK_ALPHA: f32 = 0.32;
const NODE_ALPHA: f32 = 0.6;
const PULSE_ALPHA: f32 = 0.95;
/// Added to a node or link right under the pointer, less further off.
const POINTER_ALPHA: f32 = 0.35;
/// How far the pointer's light reaches, in the screen's pixels.
const POINTER_RADIUS: f32 = 160.0;
/// Width of a link, and of a pulse's trail, in the screen's pixels.
const LINK_WIDTH: f32 = 1.0;
const TRAIL_WIDTH: f32 = 1.5;
/// Share of its link a pulse's trail covers behind it.
const TRAIL: f32 = 0.22;
/// Radius of a node with one link, and what each further link adds, in the
/// screen's pixels: a hub is drawn largest.
const NODE_RADIUS: f32 = 2.5;
const NODE_GROWTH: f32 = 0.9;
/// Radius of a pulse's head, and of the flash where it arrives, as a multiple
/// of the node's own radius.
const PULSE_RADIUS: f32 = 2.5;
const FLASH_SCALE: f32 = 2.0;
/// Corners of the polygon that draws a disc.
const DISC_CORNERS: usize = 14;
/// Seed of the pulses' choices.
const TRAFFIC_SEED: u64 = 0x5EED;

/// The setting that turns the backdrop on and off.
const SETTING: &str = "backdrop";

/// Whether the backdrop is drawn, as the setting says. A global, so that the
/// shell and the app read it in every frame without a file.
struct Shown(bool);

impl Global for Shown {}

/// Read the setting. Call once at start.
pub fn init(cx: &mut App) {
    cx.set_global(Shown(prefs::read(SETTING).as_deref() != Some("off")));
}

/// Whether the backdrop is drawn: on until the user turns it off.
pub fn shown(cx: &App) -> bool {
    cx.try_global::<Shown>().is_none_or(|shown| shown.0)
}

/// Turn the backdrop on or off, and remember it.
pub fn set_shown(on: bool, cx: &mut App) {
    prefs::write(SETTING, if on { "on" } else { "off" });
    cx.set_global(Shown(on));
}

/// Whether a wiki is being written: the app's own sync, or an update the
/// backend runs for anybody, a coding agent included.
struct Writing(bool);

impl Global for Writing {}

fn writing(cx: &App) -> bool {
    cx.try_global::<Writing>().is_some_and(|writing| writing.0)
}

/// Say whether a wiki is being written. Cheap to call often: the shell hears
/// of it only when it changes.
pub fn set_writing(on: bool, cx: &mut App) {
    if writing(cx) != on {
        cx.set_global(Writing(on));
    }
}

/// The clear margins on the left and right of the reading column, in window
/// coordinates: where the sky is drawn. The app, which lays the page out,
/// writes them; the shell reads them.
#[derive(Default)]
pub struct Margins {
    sides: Cell<Option<[Bounds<Pixels>; 2]>>,
    /// The view to draw again when they change.
    reader: Cell<Option<EntityId>>,
}

impl Margins {
    /// The two margins; `None` while the page has none.
    pub fn get(&self) -> Option<[Bounds<Pixels>; 2]> {
        self.sides.get()
    }

    fn read_by(&self, reader: EntityId) {
        self.reader.set(Some(reader));
    }

    /// Say where the margins are. Cheap to call on every frame: the reader is
    /// only told when they have changed. It runs while a frame is drawn, so
    /// the reader is told once the frame is done.
    pub fn set(&self, sides: Option<[Bounds<Pixels>; 2]>, cx: &mut App) {
        if self.sides.get() == sides {
            return;
        }
        self.sides.set(sides);
        if let Some(reader) = self.reader.get() {
            doc_view::notify_after_draw(reader, cx);
        }
    }
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
    (phase + step(seconds) / PERIOD).rem_euclid(1.0)
}

/// The time one frame moves things on by: `seconds`, never backwards and
/// never more than `MAX_STEP`.
fn step(seconds: f32) -> f32 {
    seconds.clamp(0.0, MAX_STEP)
}

/// Draws the window's title bar.
pub type TitleBarFn = Box<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// The window's outermost view: the drifting shapes, and `content` over them.
pub struct Shell<V: Render> {
    content: Entity<V>,
    /// The title bar, drawn over the top of `content`, which leaves the
    /// place empty. See the module's note for why it is not part of it.
    title_bar: TitleBarFn,
    /// The generated page, and where the app wants it.
    page: Entity<DocPage>,
    slot: Rc<PageSlot>,
    /// Where the clear margins are, which the sky shows through.
    margins: Rc<Margins>,
    sky: Constellation,
    traffic: Traffic,
    /// Seconds the sky has moved for: the pulses' clock. It stands still
    /// whenever the shapes do.
    clock: f32,
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
    /// Notches of Ctrl+wheel not yet made into a zoom step.
    wheel: Rc<RefCell<zoom::Wheel>>,
    _activation: Subscription,
    _zoom_chip: Subscription,
    _shown: Subscription,
    _writing: Subscription,
}

impl<V: Render> Shell<V> {
    pub fn new(
        content: Entity<V>,
        title_bar: TitleBarFn,
        page: Entity<DocPage>,
        slot: Rc<PageSlot>,
        margins: Rc<Margins>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        slot.read_by(cx.entity_id());
        margins.read_by(cx.entity_id());
        // Start and stop drifting as the window gains and loses focus.
        let activation = cx.observe_window_activation(window, |_, _, cx| cx.notify());
        // The zoom level in the title bar comes and goes.
        let zoom_chip = cx.observe_global::<zoom::Indicator>(|_, cx| cx.notify());
        // The setting, and a wiki that starts or stops being written.
        let shown = cx.observe_global::<Shown>(|_, cx| cx.notify());
        let writing = cx.observe_global::<Writing>(|_, cx| cx.notify());
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
            title_bar,
            page,
            slot,
            margins,
            sky: Constellation::new(),
            traffic: Traffic::new(TRAFFIC_SEED),
            clock: 0.0,
            debug_backdrop: glass::debug_backdrop(),
            always_drift: std::env::var_os("CODEWIKI_BACKDROP_DRIFT").is_some(),
            software,
            phase: 0.0,
            moved_at: None,
            tick_pending: false,
            wheel: Rc::default(),
            _activation: activation,
            _zoom_chip: zoom_chip,
            _shown: shown,
            _writing: writing,
        }
    }

    /// Move the shapes and the sky on by the time since the last frame, and
    /// ask for the next frame. Does nothing while they should stand still.
    /// Returns whether they move.
    fn drift(&mut self, window: &Window, cx: &mut Context<Self>) -> bool {
        let moving = shown(cx)
            && !cx.reduce_motion()
            && (self.always_drift || (!self.software && window.is_window_active()));
        if !moving {
            self.moved_at = None;
            return false;
        }
        let now = Instant::now();
        if let Some(before) = self.moved_at {
            let seconds = (now - before).as_secs_f32();
            self.phase = advance(self.phase, seconds);
            self.clock += step(seconds);
        }
        self.traffic.advance(&self.sky, self.clock, writing(cx));
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
        true
    }

    /// The sky as this frame shows it, or `None` while the page has no clear
    /// margins. The pointer lights it only while it moves.
    // In the screen's pixels, as the shapes: zoom leaves the sky alone.
    #[allow(clippy::disallowed_methods)]
    fn sky(&self, moving: bool, window: &Window, cx: &App) -> Option<Sky> {
        let sides = self.margins.get()?;
        let brand = crate::themes::Brand::of(cx.theme());
        let viewport = window.viewport_size();
        let pointer = (moving && window.is_window_hovered()).then(|| window.mouse_position());
        Some(Sky::new(
            &self.sky,
            &self.traffic.pulses,
            self.phase,
            self.clock,
            (viewport.width / px(1.), viewport.height / px(1.)),
            pointer.map(|p| (p.x / px(1.), p.y / px(1.))),
            [brand.primary, brand.secondary, brand.tertiary],
            sides,
        ))
    }

    // Shares of the window, in the screen's pixels: zoom leaves them alone.
    #[allow(clippy::disallowed_methods)]
    fn shapes(&self, window: &Window, cx: &App) -> Div {
        let theme = cx.theme();
        // The glow behind the window blends the theme's three brand colors.
        let brand = crate::themes::Brand::of(theme);
        let colors = [
            brand.primary,
            brand.secondary,
            brand.tertiary,
            brand.primary,
        ];
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
        // gpui-kit writes a theme file's sizes back into the theme when its
        // list of themes changes. Put the zoom back into them before anything
        // is laid out; the rem of this frame was already taken from them.
        if zoom::sync_theme(cx) {
            window.set_rem_size(cx.theme().font_size);
        }
        let moving = self.drift(window, cx);
        let shown = shown(cx);
        let sky = shown.then(|| self.sky(moving, window, cx)).flatten();
        // The title bar keeps its size at every zoom, as a browser's does.
        let title_bar = zoom::unscaled(|| (self.title_bar)(window, cx));
        let wheel = self.wheel.clone();
        div()
            .size_full()
            .relative()
            // Ctrl+wheel zooms wherever the pointer is. The wheel is taken
            // before anything under the pointer sees it, so nothing scrolls
            // and the graph does not zoom itself as well.
            .child(
                canvas(
                    |_, _, _| (),
                    move |_: Bounds<Pixels>, _, window, _| {
                        window.on_mouse_event(move |event: &ScrollEvent, phase, _, cx| {
                            let keys = &event.modifiers;
                            let zoom_keys =
                                keys.control && !keys.alt && !keys.shift && !keys.platform;
                            if phase != DispatchPhase::Capture || !zoom_keys {
                                return;
                            }
                            let notches = zoom::notches(event.delta, win32::wheel_scroll_lines());
                            let steps = wheel.borrow_mut().feed(notches, Instant::now());
                            if steps != 0 {
                                zoom::step_by(steps, cx);
                            }
                            cx.stop_propagation();
                        });
                    },
                )
                .absolute()
                .inset_0(),
            )
            // Painted first, so under everything.
            .when_some(self.debug_backdrop, |shell, color| {
                shell.child(div().absolute().inset_0().bg(color))
            })
            .when(shown, |shell| shell.child(self.shapes(window, cx)))
            .when_some(sky, |shell, sky| {
                shell.child(
                    canvas(|_, _, _| (), move |_, _, window, _| sky.paint(window))
                        .absolute()
                        .inset_0(),
                )
            })
            .child(
                self.content
                    .clone()
                    .cached(StyleRefinement::default().size_full()),
            )
            // Never cached, so that its buttons always answer.
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .w_full()
                    .child(zoom::FixedRem::new(zoom::base_rem(cx), title_bar)),
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

/// A point in the screen's pixels, from the window's top left corner.
type At = (f32, f32);

fn lerp(from: At, to: At, t: f32) -> At {
    (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t)
}

/// A node as one frame draws it.
struct Spot {
    at: At,
    radius: f32,
    cluster: usize,
    /// How much the pointer lights it, from 0 to 1.
    lit: f32,
}

/// A link as one frame draws it.
struct Line {
    from: At,
    to: At,
    cluster: usize,
    lit: f32,
}

/// A pulse as one frame draws it: its head, the trail behind it, and the
/// flash of the node it is about to reach.
struct Glint {
    tail: At,
    head: At,
    brightness: f32,
    cluster: usize,
    flash_at: At,
    flash_radius: f32,
    flash: f32,
}

/// The sky of one frame, in the screen's pixels, ready to paint.
struct Sky {
    colors: [Hsla; CLUSTERS],
    nodes: Vec<Spot>,
    links: Vec<Line>,
    glints: Vec<Glint>,
    /// The margins it shows in; nothing of it is painted elsewhere.
    sides: [Bounds<Pixels>; 2],
}

impl Sky {
    /// Lay `sky` out at `phase` in a window `size` big, with `pulses` at
    /// `clock` and the pointer, when it lights anything, at `pointer`.
    #[allow(clippy::too_many_arguments)]
    fn new(
        sky: &Constellation,
        pulses: &[Pulse],
        phase: f32,
        clock: f32,
        size: (f32, f32),
        pointer: Option<At>,
        colors: [Hsla; CLUSTERS],
        sides: [Bounds<Pixels>; 2],
    ) -> Self {
        let places: Vec<At> = sky
            .nodes
            .iter()
            .map(|node| node.place(phase, size.0, size.1))
            .collect();
        let near = |distance: f32| constellation::lit(distance, POINTER_RADIUS);
        let nodes: Vec<Spot> = sky
            .nodes
            .iter()
            .zip(&places)
            .map(|(node, &at)| Spot {
                at,
                radius: NODE_RADIUS + NODE_GROWTH * node.degree.saturating_sub(1) as f32,
                cluster: node.cluster,
                lit: pointer.map_or(0.0, |p| near((p.0 - at.0).hypot(p.1 - at.1))),
            })
            .collect();
        let links = sky
            .links
            .iter()
            .map(|&[a, b]| {
                let (from, to) = (places[a], places[b]);
                Line {
                    from,
                    to,
                    cluster: sky.nodes[a].cluster,
                    lit: pointer
                        .map_or(0.0, |p| near(constellation::segment_distance(p, from, to))),
                }
            })
            .collect();
        let glints = pulses
            .iter()
            .map(|pulse| {
                let t = pulse.progress(clock);
                let to = pulse.to(sky);
                let (start, end) = (places[pulse.from], places[to]);
                Glint {
                    tail: lerp(start, end, (t - TRAIL).max(0.0)),
                    head: lerp(start, end, t),
                    brightness: Pulse::brightness(t),
                    cluster: sky.nodes[pulse.from].cluster,
                    flash_at: end,
                    flash_radius: nodes[to].radius * FLASH_SCALE,
                    flash: Pulse::flash(t),
                }
            })
            .collect();
        Self {
            colors,
            nodes,
            links,
            glints,
            sides,
        }
    }

    fn paint(&self, window: &mut Window) {
        for side in self.sides {
            window.with_content_mask(Some(ContentMask { bounds: side }), |window| {
                self.paint_once(window)
            });
        }
    }

    // The sky keeps the screen's pixels at every zoom, as the shapes do.
    #[allow(clippy::disallowed_methods)]
    fn paint_once(&self, window: &mut Window) {
        let at = |(x, y): At| point(px(x), px(y));
        let stroke = |width: f32, from: At, to: At| {
            let mut line = PathBuilder::stroke(px(width));
            line.move_to(at(from));
            line.line_to(at(to));
            line.build().ok()
        };
        let dot = |centre: At, radius: f32| {
            let mut shape = PathBuilder::fill();
            shape.add_polygon(&disc(centre, radius), true);
            shape.build().ok()
        };
        // Links, then nodes over their ends: one path for each cluster, so
        // that the whole sky costs six draws.
        for (cluster, &color) in self.colors.iter().enumerate() {
            let mut lines = PathBuilder::stroke(px(LINK_WIDTH));
            let mut any = false;
            for line in self.links.iter().filter(|line| line.cluster == cluster) {
                lines.move_to(at(line.from));
                lines.line_to(at(line.to));
                any = true;
            }
            if any && let Ok(path) = lines.build() {
                window.paint_path(path, color.opacity(LINK_ALPHA));
            }
        }
        for (cluster, &color) in self.colors.iter().enumerate() {
            let mut discs = PathBuilder::fill();
            let mut any = false;
            for node in self.nodes.iter().filter(|node| node.cluster == cluster) {
                discs.add_polygon(&disc(node.at, node.radius), true);
                any = true;
            }
            if any && let Ok(path) = discs.build() {
                window.paint_path(path, color.opacity(NODE_ALPHA));
            }
        }
        // What the pointer lights, each as brightly as it is near.
        for line in self.links.iter().filter(|line| line.lit > 0.0) {
            if let Some(path) = stroke(LINK_WIDTH, line.from, line.to) {
                let color = self.colors[line.cluster];
                window.paint_path(path, color.opacity(POINTER_ALPHA * line.lit));
            }
        }
        for node in self.nodes.iter().filter(|node| node.lit > 0.0) {
            if let Some(path) = dot(node.at, node.radius) {
                let color = self.colors[node.cluster];
                window.paint_path(path, color.opacity(POINTER_ALPHA * node.lit));
            }
        }
        for glint in &self.glints {
            let color = self.colors[glint.cluster];
            if glint.flash > 0.0
                && let Some(path) = dot(glint.flash_at, glint.flash_radius)
            {
                window.paint_path(path, color.opacity(PULSE_ALPHA * 0.5 * glint.flash));
            }
            if glint.brightness <= 0.0 {
                continue;
            }
            if let Some(path) = stroke(TRAIL_WIDTH, glint.tail, glint.head) {
                window.paint_path(path, color.opacity(PULSE_ALPHA * 0.5 * glint.brightness));
            }
            if let Some(path) = dot(glint.head, PULSE_RADIUS) {
                window.paint_path(path, color.opacity(PULSE_ALPHA * glint.brightness));
            }
        }
    }
}

/// The corners of a polygon that draws a disc.
#[allow(clippy::disallowed_methods)]
fn disc((x, y): At, radius: f32) -> [Point<Pixels>; DISC_CORNERS] {
    std::array::from_fn(|corner| {
        let angle = TAU * corner as f32 / DISC_CORNERS as f32;
        point(px(x + radius * angle.cos()), px(y + radius * angle.sin()))
    })
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
