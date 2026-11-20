//! Zoom: the whole app larger or smaller, as a browser zooms a page.
//!
//! Ctrl+wheel, Ctrl+= and Ctrl+- step through [`LEVELS`]; Ctrl+0 goes back to
//! 100%. The level is remembered between runs.
//!
//! GPUI cannot change a window's scale: the display decides it. What GPUI
//! offers is the rem, the size its `text_sm`, `p_2` and the like are measured
//! in, which gpui-kit's root sets from the theme's font size on every frame.
//! Most of the app is measured in pixels, though. So zoom works in two
//! places:
//!
//! - The app's own sizes. A size written as an `f32` is in design units, and
//!   [`px`] is the one way to turn it into GPUI's pixels: it multiplies by
//!   the zoom. The app's modules use this `px` in place of GPUI's. A pixel
//!   value turned back into a number (`width / px(1.)`) is then in design
//!   units too, so measuring and drawing agree at every level.
//! - The theme. [`sync_theme`] scales its font sizes and radii, which moves
//!   the rem and with it every gpui-kit control.
//!
//! The title bar stays as it is, as a browser's does: it is built at 100%
//! ([`unscaled`]) and laid out in the unzoomed rem ([`FixedRem`]).

// This module turns design units into GPUI's pixels, with GPUI's `px`.
#![allow(clippy::disallowed_methods)]

use std::cell::Cell;
use std::panic::Location;
use std::time::{Duration, Instant};

use gpui_kit::component::{ActiveTheme as _, Theme};
use gpui_kit::{
    AnyElement, App, Bounds, Element, ElementId, Global, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, ScrollDelta, Window,
};

use crate::mui::motion::{LEVEL_HOLD, scaled};
use crate::prefs;

/// The levels, in percent: Chrome's zoom levels from 50% to 200%.
pub const LEVELS: [u16; 11] = [50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200];

/// The level the app starts at until the user picks another.
pub const DEFAULT: u16 = 100;

/// Name of the setting in `prefs`.
const SETTING: &str = "zoom";

/// A wheel pause longer than this starts a new gesture.
const WHEEL_IDLE: Duration = Duration::from_millis(400);

/// A precise scroll (two fingers on a touchpad, with Ctrl held) this far
/// counts as one notch of a wheel.
const PIXELS_PER_NOTCH: f32 = 50.0;

thread_local! {
    /// The zoom as a factor. Thread-local because only the main thread
    /// draws; each test runs on a thread of its own and sees 1.
    static FACTOR: Cell<f32> = const { Cell::new(1.0) };
}

/// The zoom as a factor, 1 at 100%.
pub fn factor() -> f32 {
    FACTOR.with(Cell::get)
}

fn set_factor(factor: f32) {
    FACTOR.with(|cell| cell.set(factor));
}

/// `n` design units in GPUI's pixels, at the current zoom.
#[inline]
pub fn px(n: f32) -> Pixels {
    gpui_kit::px(n * factor())
}

/// Run `build` at 100%: what it measures with [`px`] is not zoomed. The
/// zoom comes back afterwards, even if `build` panics.
pub fn unscaled<R>(build: impl FnOnce() -> R) -> R {
    with_factor(1.0, build)
}

/// Run `build` as if the zoom were `factor`, for a test of what must not
/// change with it.
#[cfg(test)]
pub fn at_factor<R>(factor: f32, build: impl FnOnce() -> R) -> R {
    with_factor(factor, build)
}

fn with_factor<R>(factor_now: f32, build: impl FnOnce() -> R) -> R {
    struct Restore(f32);
    impl Drop for Restore {
        fn drop(&mut self) {
            set_factor(self.0);
        }
    }
    let _restore = Restore(factor());
    set_factor(factor_now);
    build()
}

/// The level stored under `text`, if it is one.
pub fn parse(text: &str) -> Option<u16> {
    text.trim()
        .parse()
        .ok()
        .filter(|percent| LEVELS.contains(percent))
}

/// How a level reads: `125%`.
pub fn label(percent: u16) -> String {
    format!("{percent}%")
}

/// The level `steps` steps from `percent`, up for a positive count. A value
/// between two levels counts as already part of the way to the next one, so
/// its first step lands on that one. The ends hold.
pub fn step(percent: u16, steps: i32) -> u16 {
    let last = LEVELS.len() as i64 - 1;
    let start = match LEVELS.binary_search(&percent) {
        Ok(index) => index as i64,
        // Between `index - 1` and `index`.
        Err(index) if steps > 0 => index as i64 - 1,
        Err(index) => index as i64,
    };
    LEVELS[(start + i64::from(steps)).clamp(0, last) as usize]
}

/// How many wheel notches `delta` is, up for positive. Windows reports a
/// notch as `lines_per_notch` lines (the "lines to scroll" of its mouse
/// settings).
pub fn notches(delta: ScrollDelta, lines_per_notch: u32) -> f32 {
    match delta {
        ScrollDelta::Lines(lines) if lines_per_notch > 0 => lines.y / lines_per_notch as f32,
        // Scrolling turned off in Windows reports no lines at all.
        ScrollDelta::Lines(_) => 0.0,
        ScrollDelta::Pixels(pixels) => pixels.y / gpui_kit::px(PIXELS_PER_NOTCH),
    }
}

/// Turns wheel notches into whole steps. A high-resolution wheel or a
/// touchpad sends parts of a notch; they add up.
#[derive(Debug, Default)]
pub struct Wheel {
    total: f32,
    last: Option<Instant>,
}

impl Wheel {
    /// Add `notches` that arrived at `now`, and take the whole steps they
    /// make. A turn the other way, or a pause, starts again from zero.
    pub fn feed(&mut self, notches: f32, now: Instant) -> i32 {
        let paused = self
            .last
            .is_some_and(|last| now.saturating_duration_since(last) > WHEEL_IDLE);
        if paused || self.total * notches < 0.0 {
            self.total = 0.0;
        }
        self.last = Some(now);
        self.total += notches;
        let steps = self.total.trunc();
        self.total -= steps;
        steps as i32
    }
}

/// The level in effect.
struct Level(u16);

impl Global for Level {}

/// The chip in the title bar that shows the level after a change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Indicator {
    pub shown: bool,
    /// Counts the changes, so that the timer of an earlier one does not hide
    /// the chip of a later one.
    seq: u64,
}

impl Global for Indicator {}

/// The theme's sizes that zoom scales.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Sizes {
    font: Pixels,
    mono: Pixels,
    radius: Pixels,
    radius_lg: Pixels,
}

impl Sizes {
    fn of(theme: &Theme) -> Self {
        Self {
            font: theme.font_size,
            mono: theme.mono_font_size,
            radius: theme.radius,
            radius_lg: theme.radius_lg,
        }
    }

    fn write(self, theme: &mut Theme) {
        theme.font_size = self.font;
        theme.mono_font_size = self.mono;
        theme.radius = self.radius;
        theme.radius_lg = self.radius_lg;
    }

    fn scaled(self, factor: f32) -> Self {
        Self {
            font: self.font * factor,
            mono: self.mono * factor,
            radius: self.radius * factor,
            radius_lg: self.radius_lg * factor,
        }
    }
}

/// The theme's sizes at 100%, and what zoom last wrote into the theme.
#[derive(Debug, Clone, Copy)]
struct ThemeSizes {
    base: Sizes,
    applied: Sizes,
}

impl Global for ThemeSizes {}

/// What the theme should hold at `factor`, and the base to remember, given
/// what it holds now and what zoom remembers. A theme that no longer holds
/// what zoom wrote was changed by someone else: its sizes are the new base.
fn rebased(current: Sizes, remembered: Option<ThemeSizes>, factor: f32) -> ThemeSizes {
    let base = match remembered {
        Some(sizes) if sizes.applied == current => sizes.base,
        _ => current,
    };
    ThemeSizes {
        base,
        applied: base.scaled(factor),
    }
}

/// Apply the remembered level. Call after `themes::init`, so that the
/// theme's sizes are still at 100% when they are taken as the base.
pub fn init(cx: &mut App) {
    let percent = prefs::read(SETTING)
        .and_then(|text| parse(&text))
        .unwrap_or(DEFAULT);
    cx.set_global(Level(percent));
    set_factor(f32::from(percent) / 100.0);
    sync_theme(cx);
}

/// The level in effect, in percent.
pub fn percent(cx: &App) -> u16 {
    cx.try_global::<Level>().map_or(DEFAULT, |level| level.0)
}

/// Go to `percent`, apply it, remember it, and show it in the title bar.
pub fn set(percent: u16, cx: &mut App) {
    let percent = if LEVELS.contains(&percent) {
        percent
    } else {
        step(percent, 0)
    };
    cx.set_global(Level(percent));
    set_factor(f32::from(percent) / 100.0);
    sync_theme(cx);
    prefs::write(SETTING, &percent.to_string());
    show_indicator(cx);
    // Every view is drawn again, the cached ones too.
    cx.refresh_windows();
}

/// Go `steps` levels up, or down for a negative count.
pub fn step_by(steps: i32, cx: &mut App) {
    set(step(percent(cx), steps), cx);
}

/// Go back to 100%.
pub fn reset(cx: &mut App) {
    set(DEFAULT, cx);
}

/// Whether the level chip is shown.
pub fn indicator_shown(cx: &App) -> bool {
    cx.try_global::<Indicator>().is_some_and(|chip| chip.shown)
}

fn show_indicator(cx: &mut App) {
    let seq = cx.try_global::<Indicator>().map_or(0, |chip| chip.seq) + 1;
    cx.set_global(Indicator { shown: true, seq });
    cx.spawn(async move |cx| {
        cx.background_executor().timer(scaled(LEVEL_HOLD)).await;
        cx.update(|cx| {
            if cx.try_global::<Indicator>().map(|chip| chip.seq) == Some(seq) {
                cx.set_global(Indicator { shown: false, seq });
            }
        });
    })
    .detach();
}

/// Make the theme's sizes those of the current level. Cheap when they are
/// already, so it runs on every frame: gpui-kit writes the theme file's
/// sizes back into the theme when its list of themes changes, and that can
/// happen after zoom has scaled them. Returns whether it changed anything.
pub fn sync_theme(cx: &mut App) -> bool {
    let current = Sizes::of(cx.theme());
    let remembered = cx.try_global::<ThemeSizes>().copied();
    let sizes = rebased(current, remembered, factor());
    let unchanged = sizes.applied == current;
    if unchanged && remembered.is_some_and(|old| old.base == sizes.base) {
        return false;
    }
    if !unchanged {
        sizes.applied.write(Theme::global_mut(cx));
    }
    cx.set_global(sizes);
    !unchanged
}

/// Take the theme's sizes as they are now as its sizes at 100%, and scale
/// them. Call after a theme was applied: its sizes are then unzoomed.
pub fn rebase_theme(cx: &mut App) {
    let current = Sizes::of(cx.theme());
    let sizes = rebased(current, None, factor());
    if sizes.applied != current {
        sizes.applied.write(Theme::global_mut(cx));
    }
    cx.set_global(sizes);
}

/// The rem at 100%: the theme's font size before zoom.
pub fn base_rem(cx: &App) -> Pixels {
    cx.try_global::<ThemeSizes>()
        .map_or(cx.theme().font_size, |sizes| sizes.base.font)
}

/// Lays out and draws `child` with the rem at `rem`, whatever the window's
/// is. GPUI resolves rems while it lays out and draws, not when an element
/// is built, so a size in rems follows the rem in effect at that time.
pub struct FixedRem {
    rem: Pixels,
    child: AnyElement,
}

impl FixedRem {
    pub fn new(rem: Pixels, child: impl IntoElement) -> Self {
        Self {
            rem,
            child: child.into_any_element(),
        }
    }
}

impl IntoElement for FixedRem {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for FixedRem {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let child = &mut self.child;
        let layout =
            window.with_rem_size(Some(self.rem), |window| child.request_layout(window, cx));
        (layout, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let child = &mut self.child;
        window.with_rem_size(Some(self.rem), |window| {
            child.prepaint(window, cx);
        });
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let child = &mut self.child;
        window.with_rem_size(Some(self.rem), |window| child.paint(window, cx));
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::point;

    use super::*;

    #[test]
    fn the_levels_rise_and_hold_the_default() {
        assert!(LEVELS.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(LEVELS.contains(&DEFAULT));
    }

    #[test]
    fn a_step_moves_one_level_and_the_ends_hold() {
        assert_eq!(step(100, 1), 110);
        assert_eq!(step(100, -1), 90);
        assert_eq!(step(100, 3), 150);
        assert_eq!(step(200, 1), 200);
        assert_eq!(step(50, -1), 50);
        assert_eq!(step(100, 100), 200);
        assert_eq!(step(100, -100), 50);
        assert_eq!(step(125, 0), 125);
    }

    #[test]
    fn a_value_between_levels_steps_to_its_neighbours() {
        // 100 < 105 < 110.
        assert_eq!(step(105, 1), 110);
        assert_eq!(step(105, -1), 100);
        assert_eq!(step(30, 1), 50);
        assert_eq!(step(30, -1), 50);
        assert_eq!(step(500, -1), 200);
        assert_eq!(step(500, 1), 200);
    }

    #[test]
    fn every_level_survives_being_stored_and_nothing_else_is_taken() {
        for level in LEVELS {
            assert_eq!(parse(&level.to_string()), Some(level));
        }
        assert_eq!(parse(" 125\n"), Some(125));
        for bad in ["", "105", "0", "-100", "125%", "big", "1e2"] {
            assert_eq!(parse(bad), None, "{bad}");
        }
        assert_eq!(label(125), "125%");
    }

    #[test]
    fn a_notch_of_the_wheel_is_one_step() {
        let mut wheel = Wheel::default();
        let start = Instant::now();
        let notch = |lines: f32| notches(ScrollDelta::Lines(point(0.0, lines)), 3);
        assert_eq!(wheel.feed(notch(3.0), start), 1);
        assert_eq!(wheel.feed(notch(-3.0), start), -1);
        // A wheel set to one screen a notch reports a huge count of lines
        // (GPUI multiplies by the setting, which is then `u32::MAX`).
        let page = notches(ScrollDelta::Lines(point(0.0, u32::MAX as f32)), u32::MAX);
        assert!((page - 1.0).abs() < 0.01, "{page}");
        // Scrolling turned off zooms nothing.
        assert_eq!(notches(ScrollDelta::Lines(point(0.0, 0.0)), 0), 0.0);
    }

    #[test]
    fn parts_of_a_notch_add_up_until_the_wheel_turns_back_or_rests() {
        let mut wheel = Wheel::default();
        let start = Instant::now();
        let soon = start + Duration::from_millis(50);
        assert_eq!(wheel.feed(0.4, start), 0);
        assert_eq!(wheel.feed(0.4, soon), 0);
        assert_eq!(wheel.feed(0.4, soon), 1);
        // The rest of the last notch is dropped when the wheel turns back.
        assert_eq!(wheel.feed(-0.6, soon), 0);
        assert_eq!(wheel.feed(-0.6, soon), -1);
        // And after a pause.
        assert_eq!(wheel.feed(0.8, soon), 0);
        assert_eq!(wheel.feed(0.8, soon + WHEEL_IDLE * 2), 0);
    }

    #[test]
    fn a_precise_scroll_counts_by_distance() {
        let delta = |y: f32| ScrollDelta::Pixels(point(gpui_kit::px(0.0), gpui_kit::px(y)));
        assert_eq!(notches(delta(PIXELS_PER_NOTCH), 3), 1.0);
        assert_eq!(notches(delta(-PIXELS_PER_NOTCH / 2.0), 3), -0.5);
    }

    #[test]
    fn px_scales_and_unscaled_does_not() {
        assert_eq!(px(10.0), gpui_kit::px(10.0));
        set_factor(1.5);
        assert_eq!(px(10.0), gpui_kit::px(15.0));
        assert_eq!(unscaled(|| px(10.0)), gpui_kit::px(10.0));
        assert_eq!(factor(), 1.5);
        let caught = std::panic::catch_unwind(|| unscaled(|| panic!("in the title bar")));
        assert!(caught.is_err());
        assert_eq!(factor(), 1.5);
        set_factor(1.0);
    }

    #[test]
    fn a_size_measured_back_is_in_design_units_at_every_level() {
        for level in LEVELS {
            set_factor(f32::from(level) / 100.0);
            let width = 437.0;
            assert!((px(width) / px(1.0) - width).abs() < 1e-3, "{level}");
        }
        set_factor(1.0);
    }

    fn sizes(font: f32) -> Sizes {
        Sizes {
            font: gpui_kit::px(font),
            mono: gpui_kit::px(font - 1.0),
            radius: gpui_kit::px(6.0),
            radius_lg: gpui_kit::px(8.0),
        }
    }

    #[test]
    fn the_theme_scales_from_its_base_and_rebases_when_changed() {
        // First sight: what the theme holds is the base.
        let first = rebased(sizes(16.0), None, 1.5);
        assert_eq!(first.base, sizes(16.0));
        assert_eq!(first.applied.font, gpui_kit::px(24.0));
        assert_eq!(first.applied.radius, gpui_kit::px(9.0));
        // The theme still holds what zoom wrote: the base stays, and a new
        // level scales it, not the zoomed sizes.
        let again = rebased(first.applied, Some(first), 2.0);
        assert_eq!(again.base, sizes(16.0));
        assert_eq!(again.applied.font, gpui_kit::px(32.0));
        // Someone wrote other sizes into the theme: they are the new base.
        let changed = rebased(sizes(14.0), Some(again), 2.0);
        assert_eq!(changed.base, sizes(14.0));
        assert_eq!(changed.applied.font, gpui_kit::px(28.0));
    }
}
