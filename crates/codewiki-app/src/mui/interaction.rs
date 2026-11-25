//! How strongly a control answers the pointer: one set of numbers for hover,
//! press and selection, read by every control in `mui`.
//!
//! MUI's own strengths (`styles/createPalette.js`, the `action` object) and
//! TradingAgent's sidebar (`components/layout/Sidebar.tsx`) were too faint
//! on the app's glass, so the values in [`DARK`] are stronger. They were
//! chosen by eye on 2026-11-25, in a temporary editor beside the running app.
//!
//! [`DARK`] holds the values for a dark theme. A light theme takes them
//! through [`Interaction::for_mode`]: the state fills at half strength, as
//! MUI's light `action` table is half of its dark one, and every lift turned
//! into a darkening. Rows keep their fills in both modes, as TradingAgent's
//! sidebar does.

use gpui_kit::component::{ActiveTheme as _, Theme};
use gpui_kit::{App, Hsla};

use super::motion::{BOUNCY, Spring};
use super::tokens::{DARK_STATES, LIGHT_STATES};

/// What a light theme's state fills are of a dark theme's.
pub const LIGHT_RATIO: f32 = LIGHT_STATES.hover / DARK_STATES.hover;

/// A color a selected or switched-on control takes, from the active theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accent {
    Primary,
    /// The theme's link color.
    Secondary,
}

impl Accent {
    pub fn color(self, theme: &Theme) -> Hsla {
        match self {
            Self::Primary => theme.primary,
            Self::Secondary => theme.link,
        }
    }
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
    /// The label of a selected row, and its icon when it has no hue, and
    /// how much lighter than the accent it is drawn, so that it reads on
    /// the row's fill.
    pub selected_label: Accent,
    pub label_lift: f32,

    /// The bar of a selected row: its width, the strength of its glow
    /// against TradingAgent's, and the spring it grows and slides in on.
    pub bar_width: f32,
    pub bar_glow: f32,
    pub bar_spring: Spring,

    /// How far a pressed button sinks.
    pub sink: f32,
    /// How much darker a pressed filled button gets.
    pub press_darken: f32,
    /// How much an icon button's icon grows and brightens under the pointer.
    pub icon_button_growth: f32,
    pub icon_button_lift: f32,
    /// The icon of a switched-on icon button, and how much lighter than the
    /// accent it is drawn, so that it stands out from its fill.
    pub toggled_icon: Accent,
    pub toggled_icon_lift: f32,
    /// Opacity of a text field's or a select's border under the pointer.
    pub outline_hover: f32,

    /// Opacity of a ripple at its start and once grown.
    pub ripple: (f32, f32),

    /// How far a tab's muted label moves toward the text color under the
    /// pointer, from 0 to 1.
    pub tab_hover_text: f32,

    /// A link under the pointer: how much brighter it gets, the opacity of
    /// its underline and its thickness.
    pub link_lift: f32,
    pub link_underline: f32,
    pub link_underline_px: f32,
}

/// The values for a dark theme, chosen by eye on 2026-11-25.
pub const DARK: Interaction = Interaction {
    hover: 0.16,
    pressed: 0.28,
    selected: 0.24,
    toggled: 0.20,
    tab_hover: 0.10,
    tab_selected: 0.14,

    row_hover: 0.14,
    row_selected: 0.18,
    row_selected_hover: 0.20,
    row_pressed: 0.30,
    row_shift: 6.0,
    row_recoil: 2.0,
    icon_growth: 0.22,
    icon_lift: 0.08,
    icon_glow: 0.45,
    selected_label: Accent::Secondary,
    label_lift: 0.16,

    bar_width: 4.0,
    bar_glow: 1.3,
    bar_spring: BOUNCY,

    sink: 1.0,
    press_darken: 0.06,
    icon_button_growth: 0.15,
    icon_button_lift: 0.10,
    toggled_icon: Accent::Primary,
    toggled_icon_lift: 0.20,
    outline_hover: 0.8,

    ripple: (0.18, 0.45),

    tab_hover_text: 0.7,

    link_lift: 0.10,
    link_underline: 1.0,
    link_underline_px: 1.5,
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
            label_lift: -self.label_lift,
            icon_button_lift: -self.icon_button_lift,
            toggled_icon_lift: -self.toggled_icon_lift,
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

    #[test]
    fn a_light_theme_takes_the_state_fills_at_mui_s_ratio() {
        assert_eq!(LIGHT_RATIO, 0.5);
        let (dark, light) = (DARK.for_mode(true), DARK.for_mode(false));
        assert_eq!(dark, DARK);
        for (dark, light) in [
            (dark.hover, light.hover),
            (dark.pressed, light.pressed),
            (dark.selected, light.selected),
            (dark.toggled, light.toggled),
            (dark.tab_hover, light.tab_hover),
            (dark.tab_selected, light.tab_selected),
        ] {
            assert_eq!(light, dark * LIGHT_RATIO);
        }
        // Rows keep their fills in both modes.
        assert_eq!(
            (light.row_hover, light.row_selected, light.row_pressed),
            (dark.row_hover, dark.row_selected, dark.row_pressed)
        );
    }

    #[test]
    fn a_light_theme_darkens_where_a_dark_one_brightens() {
        let light = DARK.for_mode(false);
        assert_eq!(light.icon_lift, -DARK.icon_lift);
        assert_eq!(light.label_lift, -DARK.label_lift);
        assert_eq!(light.icon_button_lift, -DARK.icon_button_lift);
        assert_eq!(light.toggled_icon_lift, -DARK.toggled_icon_lift);
        assert_eq!(light.link_lift, -DARK.link_lift);
    }

    #[test]
    fn each_state_of_a_row_is_stronger_than_the_one_before() {
        let look = DARK;
        assert!(look.row_hover > 0.0);
        assert!(look.row_selected > look.row_hover);
        assert!(look.row_selected_hover > look.row_selected);
        assert!(look.row_pressed > look.row_selected_hover);
        // A press only ever adds to a control's fill.
        assert!(look.pressed > look.hover);
        assert!(look.selected > look.hover);
        assert!(look.ripple.1 > look.ripple.0);
        // A row gives back part of its shift when pressed, not all of it.
        assert!(look.row_recoil > 0.0 && look.row_recoil < look.row_shift);
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
