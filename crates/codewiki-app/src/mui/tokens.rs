//! The numbers behind the components.
//!
//! Each one is copied from Material UI 6.5 or from the TradingAgent theme
//! that restyles it, and names the file it came from. Where the two differ,
//! the TradingAgent value is used. Sizes are in pixels at MUI's 16 px root.
//!
//! MUI paths are relative to `@mui/material`; TradingAgent paths to its
//! `frontend/src/theme`.

use std::time::Duration;

use gpui_kit::base::animation::cubic_bezier;
use gpui_kit::{BoxShadow, FontWeight, Hsla, hsla, point, px};

/// One step of MUI's spacing scale (`theme.spacing(1)`).
pub const SPACING: f32 = 8.0;

/// How long a transition takes. `styles/createTransitions.js`, `duration`.
pub mod duration {
    use super::Duration;

    pub const SHORTEST: Duration = Duration::from_millis(150);
    pub const SHORTER: Duration = Duration::from_millis(200);
    pub const SHORT: Duration = Duration::from_millis(250);
    pub const STANDARD: Duration = Duration::from_millis(300);
    pub const COMPLEX: Duration = Duration::from_millis(375);
    pub const ENTERING_SCREEN: Duration = Duration::from_millis(225);
    pub const LEAVING_SCREEN: Duration = Duration::from_millis(195);
}

/// The control points of MUI's easing curves. `styles/createTransitions.js`,
/// `easing`.
pub mod curve {
    pub const EASE_IN_OUT: [f32; 4] = [0.4, 0.0, 0.2, 1.0];
    pub const EASE_OUT: [f32; 4] = [0.0, 0.0, 0.2, 1.0];
    pub const EASE_IN: [f32; 4] = [0.4, 0.0, 1.0, 1.0];
    pub const SHARP: [f32; 4] = [0.4, 0.0, 0.6, 1.0];
}

/// An easing function for one of the [`curve`] constants.
pub fn easing(points: [f32; 4]) -> impl Fn(f32) -> f32 {
    cubic_bezier(points[0], points[1], points[2], points[3])
}

/// How strongly a state tints a control. `styles/createPalette.js`, the
/// `action` object of the light and the dark palette.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateOpacity {
    pub hover: f32,
    pub selected: f32,
    pub focus: f32,
    pub activated: f32,
    /// Opacity of a disabled control's text and icon.
    pub disabled: f32,
    /// Opacity of a disabled control's fill.
    pub disabled_background: f32,
}

pub const LIGHT_STATES: StateOpacity = StateOpacity {
    hover: 0.04,
    selected: 0.08,
    focus: 0.12,
    activated: 0.12,
    disabled: 0.26,
    disabled_background: 0.12,
};

pub const DARK_STATES: StateOpacity = StateOpacity {
    hover: 0.08,
    selected: 0.16,
    focus: 0.12,
    activated: 0.24,
    disabled: 0.3,
    disabled_background: 0.12,
};

/// The state opacities for a dark or a light theme.
pub fn states(dark: bool) -> StateOpacity {
    if dark { DARK_STATES } else { LIGHT_STATES }
}

/// Opacity of the three layers of an elevation shadow. `styles/shadows.js`.
const UMBRA: f32 = 0.2;
const PENUMBRA: f32 = 0.14;
const AMBIENT: f32 = 0.12;

/// The highest elevation MUI defines.
pub const MAX_ELEVATION: usize = 24;

/// `styles/shadows.js`: for each elevation, three shadows of
/// (x, y, blur, spread).
#[rustfmt::skip]
const ELEVATION: [[i8; 12]; MAX_ELEVATION + 1] = [
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 2, 1, -1, 0, 1, 1, 0, 0, 1, 3, 0],
    [0, 3, 1, -2, 0, 2, 2, 0, 0, 1, 5, 0],
    [0, 3, 3, -2, 0, 3, 4, 0, 0, 1, 8, 0],
    [0, 2, 4, -1, 0, 4, 5, 0, 0, 1, 10, 0],
    [0, 3, 5, -1, 0, 5, 8, 0, 0, 1, 14, 0],
    [0, 3, 5, -1, 0, 6, 10, 0, 0, 1, 18, 0],
    [0, 4, 5, -2, 0, 7, 10, 1, 0, 2, 16, 1],
    [0, 5, 5, -3, 0, 8, 10, 1, 0, 3, 14, 2],
    [0, 5, 6, -3, 0, 9, 12, 1, 0, 3, 16, 2],
    [0, 6, 6, -3, 0, 10, 14, 1, 0, 4, 18, 3],
    [0, 6, 7, -4, 0, 11, 15, 1, 0, 4, 20, 3],
    [0, 7, 8, -4, 0, 12, 17, 2, 0, 5, 22, 4],
    [0, 7, 8, -4, 0, 13, 19, 2, 0, 5, 24, 4],
    [0, 7, 9, -4, 0, 14, 21, 2, 0, 5, 26, 4],
    [0, 8, 9, -5, 0, 15, 22, 2, 0, 6, 28, 5],
    [0, 8, 10, -5, 0, 16, 24, 2, 0, 6, 30, 5],
    [0, 8, 11, -5, 0, 17, 26, 2, 0, 6, 32, 5],
    [0, 9, 11, -5, 0, 18, 28, 2, 0, 7, 34, 6],
    [0, 9, 12, -6, 0, 19, 29, 2, 0, 7, 36, 6],
    [0, 10, 13, -6, 0, 20, 31, 3, 0, 8, 38, 7],
    [0, 10, 13, -6, 0, 21, 33, 3, 0, 8, 40, 7],
    [0, 10, 14, -6, 0, 22, 35, 3, 0, 8, 42, 7],
    [0, 11, 14, -7, 0, 23, 36, 3, 0, 9, 44, 8],
    [0, 11, 15, -7, 0, 24, 38, 3, 0, 9, 46, 8],
];

/// The shadow of a surface raised to `level` (0 to 24). Levels above the
/// highest are drawn as the highest.
pub fn elevation(level: usize) -> Vec<BoxShadow> {
    elevation_between(level, level, 0.0)
}

/// A shadow part of the way from one elevation to another, for a surface
/// that is rising or settling. `progress` runs from 0 (`from`) to 1 (`to`)
/// and may overshoot.
pub fn elevation_between(from: usize, to: usize, progress: f32) -> Vec<BoxShadow> {
    let from = ELEVATION[from.min(MAX_ELEVATION)];
    let to = ELEVATION[to.min(MAX_ELEVATION)];
    let at = |index: usize| {
        let (a, b) = (f32::from(from[index]), f32::from(to[index]));
        a + (b - a) * progress
    };
    [UMBRA, PENUMBRA, AMBIENT]
        .into_iter()
        .enumerate()
        .map(|(layer, opacity)| {
            let base = layer * 4;
            BoxShadow {
                color: black(opacity),
                offset: point(px(at(base)), px(at(base + 1))),
                // A blur cannot be negative, whatever a spring overshoots to.
                blur_radius: px(at(base + 2).max(0.0)),
                spread_radius: px(at(base + 3)),
                inset: false,
            }
        })
        .collect()
}

fn black(opacity: f32) -> Hsla {
    hsla(0.0, 0.0, 0.0, opacity)
}

/// A text style. `typography.ts` (TradingAgent), which overrides
/// `styles/createTypography.js`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Variant {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    Subtitle1,
    Subtitle2,
    Body1,
    Body2,
    Button,
    Caption,
    Overline,
}

/// What a [`Variant`] comes to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextSpec {
    /// Font size in pixels.
    pub size: f32,
    pub weight: FontWeight,
    /// Line height as a multiple of the font size.
    pub line_height: f32,
    /// Whether the text is set in capitals.
    pub uppercase: bool,
}

impl Variant {
    pub const ALL: [Self; 13] = [
        Self::H1,
        Self::H2,
        Self::H3,
        Self::H4,
        Self::H5,
        Self::H6,
        Self::Subtitle1,
        Self::Subtitle2,
        Self::Body1,
        Self::Body2,
        Self::Button,
        Self::Caption,
        Self::Overline,
    ];

    pub fn spec(self) -> TextSpec {
        let spec = |size, weight, line_height| TextSpec {
            size,
            weight,
            line_height,
            uppercase: false,
        };
        match self {
            Self::H1 => spec(36.0, FontWeight::BOLD, 1.2),
            Self::H2 => spec(32.0, FontWeight::BOLD, 1.3),
            Self::H3 => spec(24.0, FontWeight::SEMIBOLD, 1.4),
            Self::H4 => spec(18.0, FontWeight::SEMIBOLD, 1.4),
            Self::H5 => spec(16.0, FontWeight::SEMIBOLD, 1.5),
            Self::H6 => spec(14.0, FontWeight::SEMIBOLD, 1.5),
            Self::Subtitle1 => spec(16.0, FontWeight::MEDIUM, 1.5),
            Self::Subtitle2 => spec(14.0, FontWeight::MEDIUM, 1.57),
            Self::Body1 => spec(16.0, FontWeight::NORMAL, 1.5),
            Self::Body2 => spec(14.0, FontWeight::NORMAL, 1.57),
            // TradingAgent turns off MUI's capitals and raises the weight.
            Self::Button => spec(14.0, FontWeight::SEMIBOLD, 1.75),
            Self::Caption => spec(12.0, FontWeight::NORMAL, 1.66),
            Self::Overline => TextSpec {
                uppercase: true,
                ..spec(12.0, FontWeight::MEDIUM, 2.66)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_are_the_ones_mui_names() {
        let millis = [
            duration::SHORTEST,
            duration::SHORTER,
            duration::SHORT,
            duration::STANDARD,
            duration::COMPLEX,
            duration::ENTERING_SCREEN,
            duration::LEAVING_SCREEN,
        ]
        .map(|d| d.as_millis());
        assert_eq!(millis, [150, 200, 250, 300, 375, 225, 195]);
    }

    #[test]
    fn easing_curves_start_and_end_where_they_should() {
        for points in [
            curve::EASE_IN_OUT,
            curve::EASE_OUT,
            curve::EASE_IN,
            curve::SHARP,
        ] {
            let ease = easing(points);
            assert!(ease(0.0).abs() < 1e-3, "{points:?}");
            assert!((ease(1.0) - 1.0).abs() < 1e-3, "{points:?}");
            assert!(ease(0.25) < ease(0.75), "{points:?}");
        }
        // Ease-out is ahead of ease-in at the half-way point.
        assert!(easing(curve::EASE_OUT)(0.5) > easing(curve::EASE_IN)(0.5));
    }

    #[test]
    fn a_dark_theme_tints_states_more_strongly() {
        assert_eq!(states(true), DARK_STATES);
        assert_eq!(states(false), LIGHT_STATES);
        assert_eq!((LIGHT_STATES.hover, DARK_STATES.hover), (0.04, 0.08));
        assert_eq!((LIGHT_STATES.selected, DARK_STATES.selected), (0.08, 0.16));
        assert_eq!(LIGHT_STATES.focus, DARK_STATES.focus);
    }

    #[test]
    fn elevation_has_three_layers_and_none_at_zero() {
        let flat = elevation(0);
        assert_eq!(flat.len(), 3);
        assert!(
            flat.iter()
                .all(|s| s.blur_radius == px(0.) && s.offset.y == px(0.))
        );
        // `createShadow(0, 3, 1, -2, 0, 2, 2, 0, 0, 1, 5, 0)`.
        let two = elevation(2);
        assert_eq!(two[0].offset.y, px(3.));
        assert_eq!(two[0].blur_radius, px(1.));
        assert_eq!(two[0].spread_radius, px(-2.));
        assert_eq!(two[1].offset.y, px(2.));
        assert_eq!(two[2].blur_radius, px(5.));
        assert_eq!(two[0].color.a, 0.2);
        assert_eq!(two[1].color.a, 0.14);
        assert_eq!(two[2].color.a, 0.12);
    }

    #[test]
    fn higher_elevations_cast_longer_shadows() {
        let reach = |level| {
            let shadows = elevation(level);
            shadows[1].offset.y / px(1.) + shadows[1].blur_radius / px(1.)
        };
        for level in 1..=MAX_ELEVATION {
            assert!(reach(level) > reach(level - 1), "level {level}");
        }
        assert_eq!(elevation(99), elevation(MAX_ELEVATION));
    }

    #[test]
    fn a_shadow_part_way_lies_between_its_ends() {
        assert_eq!(elevation_between(2, 8, 0.0), elevation(2));
        assert_eq!(elevation_between(2, 8, 1.0), elevation(8));
        let half = elevation_between(2, 8, 0.5);
        assert_eq!(half[1].offset.y, px(5.));
        // An overshooting spring never asks for a negative blur.
        let under = elevation_between(1, 0, 1.5);
        assert!(under.iter().all(|s| s.blur_radius >= px(0.)));
    }

    #[test]
    fn text_styles_are_the_trading_agent_ones() {
        let button = Variant::Button.spec();
        assert_eq!(button.size, 14.0);
        assert_eq!(button.weight, FontWeight::SEMIBOLD);
        assert!(!button.uppercase);
        assert_eq!(Variant::Body1.spec().size, 16.0);
        assert_eq!(Variant::Caption.spec().size, 12.0);
        assert!(Variant::Overline.spec().uppercase);
        // Headings shrink with depth.
        let sizes = [
            Variant::H1,
            Variant::H2,
            Variant::H3,
            Variant::H4,
            Variant::H5,
            Variant::H6,
        ]
        .map(|v| v.spec().size);
        assert!(sizes.windows(2).all(|pair| pair[0] > pair[1]), "{sizes:?}");
        for variant in Variant::ALL {
            let spec = variant.spec();
            assert!(spec.size >= 12.0 && spec.line_height >= 1.2, "{variant:?}");
        }
    }
}
