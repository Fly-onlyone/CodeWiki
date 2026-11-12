//! How things move: springs, entrances and stagger.
//!
//! The springs are the ones TradingAgent gives Framer Motion
//! (`frontend/src/utils/animations/variants.ts`). Framer describes a spring by
//! stiffness, damping and mass, and so does GPUI, so the values carry over as
//! they are.
//!
//! GPUI cannot scale an element, so the entrances here keep Framer's fade and
//! travel and leave out its scale.
//!
//! Set `CODEWIKI_MOTION_SCALE` to a number above 1 to stretch every motion by
//! that factor. A window capture can then catch an animation half way.

use std::sync::OnceLock;
use std::time::Duration;

use gpui_kit::base::motion as kit;
use gpui_kit::{
    Animation, AnimationElement, AnimationExt as _, ElementId, IntoElement, SpringConfig, Styled,
    px, sampled_easing,
};

/// A spring as Framer Motion describes one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring {
    pub stiffness: f32,
    pub damping: f32,
    pub mass: f32,
}

/// `spring.snappy`: the default for things that enter or follow the pointer.
pub const SNAPPY: Spring = Spring::new(400.0, 30.0);
/// `spring.bouncy`: overshoots; for dialogs and small accents.
pub const BOUNCY: Spring = Spring::new(300.0, 20.0).mass(0.8);
/// `spring.gentle`: slow and soft; for single elements and counters.
pub const GENTLE: Spring = Spring::new(200.0, 25.0);
/// The spring of `pageEntrance` and `scrollReveal`.
pub const PAGE: Spring = Spring::new(300.0, 30.0);

/// Travel of `pageEntrance`, in pixels.
pub const PAGE_RISE: f32 = 30.0;
/// Travel of a `staggerCascade` item.
pub const CARD_RISE: f32 = 40.0;
/// Travel of a `listStagger` item, from the left.
pub const ROW_SLIDE: f32 = 20.0;
/// Travel of `tabSwitch`.
pub const TAB_RISE: f32 = 10.0;

/// `staggerCascade`: time between two cards.
pub const CARD_STAGGER: Duration = Duration::from_millis(80);
/// `listStagger`: time between two rows.
pub const ROW_STAGGER: Duration = Duration::from_millis(30);
/// Items after this many enter together with the last staggered one. Without
/// a cap the last row of a long table would arrive seconds late.
pub const STAGGER_CAP: usize = 12;

/// `scaleIn` exit, and the exit of `tabSwitch` and `pageEntrance`.
pub const EXIT: Duration = Duration::from_millis(120);

/// Springs settle to within this share of their travel.
const EPSILON: f32 = 0.001;

impl Spring {
    pub const fn new(stiffness: f32, damping: f32) -> Self {
        Self {
            stiffness,
            damping,
            mass: 1.0,
        }
    }

    pub const fn mass(mut self, mass: f32) -> Self {
        self.mass = mass;
        self
    }

    /// The time one swing would take with no damping: `2π / √(k/m)`.
    pub fn response(self) -> Duration {
        Duration::from_secs_f32(std::f32::consts::TAU / (self.stiffness / self.mass).sqrt())
    }

    /// The damping ratio `c / (2√(km))`. Below 1 the spring overshoots.
    pub fn damping_ratio(self) -> f32 {
        self.damping / (2.0 * (self.stiffness * self.mass).sqrt())
    }

    /// The same motion, `factor` times slower.
    fn slowed(self, factor: f32) -> Self {
        Self {
            stiffness: self.stiffness / (factor * factor),
            damping: self.damping / factor,
            mass: self.mass,
        }
    }

    /// For `with_spring` and [`sampled_easing`].
    pub fn config(self) -> SpringConfig {
        let spring = self.slowed(scale());
        SpringConfig::new(spring.stiffness, spring.damping, spring.mass)
    }

    /// One run of the spring from rest at 0 to rest at 1, as an animation.
    pub fn animation(self) -> Animation {
        let (travel, easing) = sampled_easing(self.config(), EPSILON);
        Animation::new(travel).with_easing(easing)
    }

    /// For `gpui_kit::base::motion::spring`, which follows a moving target.
    /// `epsilon` is how close counts as arrived, in the unit of the value.
    pub fn policy(self, epsilon: f32) -> kit::Spring {
        kit::Spring::new(scaled(self.response()))
            .with_damping(self.damping_ratio())
            .with_epsilon(epsilon)
    }
}

/// The factor every motion is stretched by; 1 unless
/// `CODEWIKI_MOTION_SCALE` says otherwise.
pub fn scale() -> f32 {
    static SCALE: OnceLock<f32> = OnceLock::new();
    *SCALE.get_or_init(|| parse_scale(std::env::var("CODEWIKI_MOTION_SCALE").ok().as_deref()))
}

fn parse_scale(value: Option<&str>) -> f32 {
    value
        .and_then(|text| text.trim().parse::<f32>().ok())
        .filter(|factor| factor.is_finite() && *factor >= 1.0)
        .unwrap_or(1.0)
}

/// `duration` stretched by [`scale`].
pub fn scaled(duration: Duration) -> Duration {
    duration.mul_f32(scale())
}

/// A figure such as `286`, `62.5%` or `1,204 lines`, `progress` of the way
/// up from zero: TradingAgent's `RollingDigits`. Text that does not start
/// with a number is returned as it is.
pub fn count_up(value: &str, progress: f32) -> String {
    let digits = value
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ','))
        .unwrap_or(value.len());
    let (figure, rest) = value.split_at(digits);
    let plain: String = figure.chars().filter(|c| *c != ',').collect();
    let Ok(target) = plain.parse::<f64>() else {
        return value.to_string();
    };
    if progress >= 1.0 {
        return value.to_string();
    }
    let decimals = plain
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len());
    // A spring may dip below zero or pass its target; a count does neither.
    let shown = target * f64::from(progress.clamp(0.0, 1.0));
    format!("{shown:.decimals$}{rest}")
}

/// How long item `index` of a staggered group waits before it enters.
pub fn stagger_delay(index: usize, step: Duration) -> Duration {
    step * index.min(STAGGER_CAP) as u32
}

/// An entrance: where the element comes from, on which spring, after what
/// wait.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Enter {
    /// Offset the element starts at, in pixels.
    pub from: (f32, f32),
    pub spring: Spring,
    pub delay: Duration,
}

impl Enter {
    /// `pageEntrance`.
    pub const PAGE: Self = Self::rise(PAGE_RISE, PAGE);
    /// `tabSwitch`.
    pub const TAB: Self = Self::rise(TAB_RISE, SNAPPY);
    /// `scaleIn`, without the scale: a short drop on the bouncy spring.
    pub const DIALOG: Self = Self::rise(-12.0, BOUNCY);

    /// Comes up from `distance` pixels below.
    pub const fn rise(distance: f32, spring: Spring) -> Self {
        Self {
            from: (0.0, distance),
            spring,
            delay: Duration::ZERO,
        }
    }

    /// Comes in from `distance` pixels to the left.
    pub const fn slide(distance: f32, spring: Spring) -> Self {
        Self {
            from: (-distance, 0.0),
            spring,
            delay: Duration::ZERO,
        }
    }

    /// Item `index` of a `staggerCascade`.
    pub fn card(index: usize) -> Self {
        Self::rise(CARD_RISE, SNAPPY).after(stagger_delay(index, CARD_STAGGER))
    }

    /// Item `index` of a `listStagger`.
    pub fn row(index: usize) -> Self {
        Self::slide(ROW_SLIDE, SNAPPY).after(stagger_delay(index, ROW_STAGGER))
    }

    pub const fn after(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Where the element is and how visible, `progress` of the way in.
    /// `progress` may pass 1 while the spring overshoots; the offset follows
    /// it and the opacity does not.
    pub fn at(self, progress: f32) -> (f32, f32, f32) {
        let left = 1.0 - progress;
        (
            self.from.0 * left,
            self.from.1 * left,
            progress.clamp(0.0, 1.0),
        )
    }
}

/// Elements that can make an entrance.
pub trait EnterExt: Styled + IntoElement + Sized + 'static {
    /// Fade and travel into place when the element first appears, and again
    /// whenever `id` changes. Under reduced motion the element is simply
    /// there.
    fn enter(self, id: impl Into<ElementId>, enter: Enter) -> AnimationElement<Self> {
        // The wait is a first animation that holds the element at its start.
        let animations = vec![
            Animation::new(scaled(enter.delay)),
            enter.spring.animation(),
        ];
        self.with_animations(id, animations, move |element, stage, progress| {
            let progress = if stage == 0 { 0.0 } else { progress };
            let (x, y, opacity) = enter.at(progress);
            element.relative().left(px(x)).top(px(y)).opacity(opacity)
        })
    }
}

impl<E: Styled + IntoElement + 'static> EnterExt for E {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_springs_are_the_trading_agent_presets() {
        assert_eq!(
            SNAPPY,
            Spring {
                stiffness: 400.0,
                damping: 30.0,
                mass: 1.0
            }
        );
        assert_eq!(
            BOUNCY,
            Spring {
                stiffness: 300.0,
                damping: 20.0,
                mass: 0.8
            }
        );
        assert_eq!(GENTLE, Spring::new(200.0, 25.0));
        assert_eq!(PAGE, Spring::new(300.0, 30.0));
    }

    #[test]
    fn a_spring_converts_to_response_and_ratio() {
        // k = 400, m = 1: ω₀ = 20 rad/s, so one swing takes 2π/20 s.
        assert!((SNAPPY.response().as_secs_f32() - 0.314_16).abs() < 1e-4);
        // c / (2√(km)) = 30 / 40.
        assert!((SNAPPY.damping_ratio() - 0.75).abs() < 1e-6);
        // Only the bouncy spring is meant to overshoot visibly.
        assert!(BOUNCY.damping_ratio() < SNAPPY.damping_ratio());
        assert!(BOUNCY.damping_ratio() < 0.7);
        assert!(GENTLE.damping_ratio() > 0.85);
    }

    #[test]
    fn the_conversion_round_trips_through_gpui() {
        for spring in [SNAPPY, BOUNCY, GENTLE, PAGE] {
            let config = SpringConfig::new(spring.stiffness, spring.damping, spring.mass);
            let (frequency, ratio) = config.canonical();
            let response = std::f32::consts::TAU / frequency;
            assert!((spring.response().as_secs_f32() - response).abs() < 1e-5);
            assert!((spring.damping_ratio() - ratio).abs() < 1e-5);
        }
    }

    #[test]
    fn a_slowed_spring_keeps_its_shape() {
        let slow = SNAPPY.slowed(10.0);
        assert!((slow.damping_ratio() - SNAPPY.damping_ratio()).abs() < 1e-5);
        let ratio = slow.response().as_secs_f32() / SNAPPY.response().as_secs_f32();
        assert!((ratio - 10.0).abs() < 1e-3, "{ratio}");
        assert_eq!(SNAPPY.slowed(1.0), SNAPPY);
    }

    #[test]
    fn the_scale_is_read_strictly() {
        assert_eq!(parse_scale(None), 1.0);
        assert_eq!(parse_scale(Some("10")), 10.0);
        assert_eq!(parse_scale(Some(" 2.5 ")), 2.5);
        // Motion is never sped up, and nonsense is ignored.
        for bad in ["0.5", "0", "-3", "fast", "", "inf", "NaN"] {
            assert_eq!(parse_scale(Some(bad)), 1.0, "{bad}");
        }
    }

    #[test]
    fn a_figure_counts_up_to_itself() {
        assert_eq!(count_up("286", 0.0), "0");
        assert_eq!(count_up("286", 0.5), "143");
        assert_eq!(count_up("286", 1.0), "286");
        // Decimals and what follows the number are kept.
        assert_eq!(count_up("62.5%", 0.5), "31.2%");
        assert_eq!(count_up("62.5%", 1.0), "62.5%");
        assert_eq!(count_up("40 lines", 0.25), "10 lines");
        // The finished figure is the text it was given, separators and all.
        assert_eq!(count_up("1,204", 1.0), "1,204");
        assert_eq!(count_up("1,204", 0.5), "602");
        // An overshooting spring does not show more than the figure.
        assert_eq!(count_up("10", 1.2), "10");
        assert_eq!(count_up("10", -0.3), "0");
    }

    #[test]
    fn text_that_is_not_a_figure_does_not_count() {
        for word in ["medium", "rust", "—", "", "v2"] {
            assert_eq!(count_up(word, 0.3), word);
        }
    }

    #[test]
    fn stagger_grows_then_stops() {
        assert_eq!(stagger_delay(0, CARD_STAGGER), Duration::ZERO);
        assert_eq!(stagger_delay(3, CARD_STAGGER), Duration::from_millis(240));
        assert_eq!(stagger_delay(3, ROW_STAGGER), Duration::from_millis(90));
        let last = stagger_delay(STAGGER_CAP, ROW_STAGGER);
        assert_eq!(stagger_delay(500, ROW_STAGGER), last);
        assert!(last <= Duration::from_millis(400));
        assert!(stagger_delay(500, CARD_STAGGER) <= Duration::from_secs(1));
    }

    #[test]
    fn an_entrance_ends_in_place_and_fully_visible() {
        for enter in [Enter::PAGE, Enter::TAB, Enter::card(4), Enter::row(2)] {
            let (x, y, opacity) = enter.at(0.0);
            assert_eq!((x, y), enter.from);
            assert_eq!(opacity, 0.0);
            assert_eq!(enter.at(1.0), (0.0, 0.0, 1.0));
        }
        // An overshoot carries the element past its place, still opaque.
        let (_, y, opacity) = Enter::PAGE.at(1.1);
        assert!(y < 0.0);
        assert_eq!(opacity, 1.0);
    }

    #[test]
    fn entrances_come_from_where_framer_brings_them() {
        assert_eq!(Enter::PAGE.from, (0.0, 30.0));
        assert_eq!(Enter::card(0).from, (0.0, 40.0));
        assert_eq!(Enter::row(0).from, (-20.0, 0.0));
        assert_eq!(Enter::card(2).delay, Duration::from_millis(160));
        assert_eq!(Enter::row(2).delay, Duration::from_millis(60));
    }
}
