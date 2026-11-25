//! How strongly a control answers the pointer: one set of numbers for hover,
//! press and selection, read by every control in `mui`.
//!
//! [`DARK`] holds the values for a dark theme. A light theme takes them
//! through [`Interaction::for_mode`]: the state fills at half strength, as
//! MUI's light `action` table is half of its dark one
//! (`styles/createPalette.js`), and every lift turned into a darkening.
//! Rows keep their fills in both modes, as TradingAgent's sidebar does
//! (`components/layout/Sidebar.tsx`).

use gpui_kit::component::{ActiveTheme as _, Theme};
use gpui_kit::{App, Hsla};

use super::motion::{SNAPPY, Spring};
use super::tokens::{DARK_STATES, LIGHT_STATES};

/// What a light theme's state fills are of a dark theme's.
pub const LIGHT_RATIO: f32 = LIGHT_STATES.hover / DARK_STATES.hover;

/// A color a selected or toggled control can take, from the active theme.
// The choices are tried in the app before one is settled on; the ones not
// chosen go then.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accent {
    Foreground,
    Primary,
    /// The theme's link color.
    Secondary,
    /// The color of the selected row's bar.
    Tertiary,
}

impl Accent {
    pub fn color(self, theme: &Theme) -> Hsla {
        match self {
            Self::Foreground => theme.foreground,
            Self::Primary => theme.primary,
            Self::Secondary => theme.link,
            Self::Tertiary => theme.sidebar_primary,
        }
    }
}

/// How the bar of a selected row appears.
#[allow(dead_code)] // As `Accent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarEntry {
    /// At once.
    Instant,
    /// Grows from its middle.
    Grow,
    /// Slides in from the left.
    Slide,
    /// Both.
    GrowSlide,
}

/// The strengths. Fills are opacities of the control's color, lifts are
/// changes of lightness, lengths are in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interaction {
    /// Fill of a button, an icon button or a toggle under the pointer.
    pub hover: f32,
    /// Fill a pressed control flashes to.
    pub pressed: f32,
    /// Fill of a chosen toggle.
    pub selected: f32,
    /// Fill of an icon button that is switched on, such as the sidebar's.
    pub toggled: f32,
    /// Fill of a tab under the pointer, and of the selected tab.
    pub tab_hover: f32,
    pub tab_selected: f32,

    /// Fills of a list row.
    pub row_hover: f32,
    pub row_selected: f32,
    pub row_selected_hover: f32,
    pub row_pressed: f32,
    /// How far a row moves right under the pointer, and how much of that
    /// it gives back while pressed.
    pub row_shift: f32,
    pub row_recoil: f32,
    /// How much a row's icon grows under the pointer.
    pub icon_growth: f32,
    /// How much brighter a tinted icon gets under the pointer or selected.
    pub icon_lift: f32,
    /// Opacity of the glow around a selected row's tinted icon.
    pub icon_glow: f32,
    /// The label of a selected row, and its icon when it has no hue.
    pub selected_label: Accent,

    /// The bar of a selected row: its width, the strength of its glow
    /// against TradingAgent's, how it appears and on which spring.
    pub bar_width: f32,
    pub bar_glow: f32,
    pub bar_entry: BarEntry,
    pub bar_spring: Spring,

    /// How far a pressed button sinks.
    pub sink: f32,
    /// How much darker a pressed filled button gets.
    pub press_darken: f32,
    /// How much an icon button's icon grows and brightens under the pointer.
    pub icon_button_growth: f32,
    pub icon_button_lift: f32,
    /// The icon of a switched-on icon button; Foreground keeps its own.
    pub toggled_icon: Accent,
    /// Opacity of a text field's or a select's border under the pointer.
    pub outline_hover: f32,

    /// Opacity of a ripple at its start and once grown.
    pub ripple: (f32, f32),

    /// How far a tab's muted label moves toward the text color under the
    /// pointer, from 0 to 1, and whether the selected tab is semibold.
    pub tab_hover_text: f32,
    pub tab_semibold: bool,

    /// A link under the pointer: how much brighter it gets, the opacity of
    /// its underline (0 leaves the underline as it is) and its thickness.
    pub link_lift: f32,
    pub link_underline: f32,
    pub link_underline_px: f32,
}

/// The values for a dark theme.
pub const DARK: Interaction = Interaction {
    hover: DARK_STATES.hover,
    pressed: 0.0,
    selected: DARK_STATES.selected,
    toggled: 0.0,
    tab_hover: 0.0,
    tab_selected: 0.0,

    // `Sidebar.tsx`: how strongly the primary color fills a row.
    row_hover: 0.08,
    row_selected: 0.1,
    row_selected_hover: 0.15,
    row_pressed: 0.0,
    // `Sidebar.tsx`: `translateX(4px)` and an icon at 115% under the pointer.
    row_shift: 4.0,
    row_recoil: 0.0,
    icon_growth: 0.15,
    icon_lift: 0.0,
    icon_glow: 0.0,
    selected_label: Accent::Foreground,

    // `Sidebar.tsx`: the bar of the selected row is 3 px wide.
    bar_width: 3.0,
    bar_glow: 1.0,
    bar_entry: BarEntry::Instant,
    bar_spring: SNAPPY,

    sink: 0.0,
    press_darken: 0.0,
    icon_button_growth: 0.0,
    icon_button_lift: 0.0,
    toggled_icon: Accent::Foreground,
    // TradingAgent, `MuiTextField`: the border under the pointer is the
    // primary color at half strength.
    outline_hover: 0.5,

    // `TouchRipple.js`, `enterKeyframe`.
    ripple: (0.1, 0.3),

    tab_hover_text: 0.0,
    tab_semibold: false,

    link_lift: 0.0,
    link_underline: 0.0,
    link_underline_px: 1.0,
};

impl Interaction {
    /// The values for a dark or a light theme.
    pub fn for_mode(self, dark: bool) -> Self {
        if dark {
            return self;
        }
        Self {
            hover: self.hover * LIGHT_RATIO,
            pressed: self.pressed * LIGHT_RATIO,
            selected: self.selected * LIGHT_RATIO,
            toggled: self.toggled * LIGHT_RATIO,
            tab_hover: self.tab_hover * LIGHT_RATIO,
            tab_selected: self.tab_selected * LIGHT_RATIO,
            icon_lift: -self.icon_lift,
            icon_button_lift: -self.icon_button_lift,
            link_lift: -self.link_lift,
            ..self
        }
    }
}

/// The values in effect for the active theme.
pub fn current(cx: &App) -> Interaction {
    DARK.for_mode(cx.theme().is_dark())
}

/// `color` made `amount` lighter, or darker for a negative amount. Hue and
/// saturation stay, so a tinted icon keeps its kind.
pub fn lift(color: Hsla, amount: f32) -> Hsla {
    Hsla {
        l: (color.l + amount).clamp(0.0, 1.0),
        ..color
    }
}

/// `rest` on its way to the pressed fill `alpha`, `progress` of the way. A
/// fill already stronger than the pressed one stays as it is.
pub fn pressed(rest: Hsla, alpha: f32, progress: f32) -> Hsla {
    let gain = (alpha - rest.a).max(0.0) * progress.clamp(0.0, 1.0);
    Hsla {
        a: rest.a + gain,
        ..rest
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::hsla;

    use super::*;
    use crate::mui::tokens;

    #[test]
    fn the_state_fills_are_mui_s_in_both_modes() {
        let dark = DARK.for_mode(true);
        assert_eq!(dark.hover, tokens::DARK_STATES.hover);
        assert_eq!(dark.selected, tokens::DARK_STATES.selected);
        let light = DARK.for_mode(false);
        assert_eq!(light.hover, tokens::LIGHT_STATES.hover);
        assert_eq!(light.selected, tokens::LIGHT_STATES.selected);
    }

    #[test]
    fn a_row_fills_as_trading_agent_fills_it_in_both_modes() {
        for dark in [true, false] {
            let look = DARK.for_mode(dark);
            assert_eq!(
                (look.row_hover, look.row_selected, look.row_selected_hover),
                (0.08, 0.1, 0.15)
            );
        }
    }

    #[test]
    fn a_light_theme_darkens_where_a_dark_one_brightens() {
        let look = Interaction {
            icon_lift: 0.1,
            icon_button_lift: 0.2,
            link_lift: 0.3,
            ..DARK
        };
        let light = look.for_mode(false);
        assert_eq!(
            (light.icon_lift, light.icon_button_lift, light.link_lift),
            (-0.1, -0.2, -0.3)
        );
    }

    #[test]
    fn a_lift_keeps_hue_and_saturation() {
        let tint = hsla(0.15, 0.9, 0.6, 1.0);
        for amount in [-0.8, -0.1, 0.0, 0.1, 0.8] {
            let lifted = lift(tint, amount);
            assert_eq!((lifted.h, lifted.s, lifted.a), (tint.h, tint.s, tint.a));
            assert!((0.0..=1.0).contains(&lifted.l));
        }
        assert!(lift(tint, 0.1).l > tint.l);
    }

    #[test]
    fn a_press_only_ever_strengthens_a_fill() {
        let rest = hsla(0.7, 0.8, 0.6, 0.1);
        assert_eq!(pressed(rest, 0.3, 0.0), rest);
        assert!((pressed(rest, 0.3, 1.0).a - 0.3).abs() < 1e-6);
        assert!((pressed(rest, 0.3, 0.5).a - 0.2).abs() < 1e-6);
        // A press weaker than the fill leaves it, and so does none at all.
        assert_eq!(pressed(rest, 0.05, 1.0), rest);
        assert_eq!(pressed(rest, 0.0, 1.0), rest);
        assert_eq!(pressed(rest, 0.3, 1.0).h, rest.h);
    }
}
