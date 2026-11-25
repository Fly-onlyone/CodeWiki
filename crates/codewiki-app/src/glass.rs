//! Glass: how see-through each surface is.
//!
//! The window sits on the Windows Mica backdrop and every surface is the
//! theme's color at less than full opacity. gpui-kit cannot blur inside a
//! window, so the backdrop is the only source of the glass look.
//!
//! Two kinds of surface exist. The *pane* carries the page: long text that
//! must stay readable whatever is behind the window. The *chrome* is
//! everything around it — title bar, sidebar, toolbar, tabs, details — and
//! shows more of the backdrop.
//!
//! How opaque each must be is worked out from the theme, not fixed: a surface
//! is as clear as it can be while its text keeps the contrast `contrast`
//! asks for. A theme with weak contrast therefore gets denser glass.
//!
//! A third kind is the *margin*: the empty sides of a page beside its reading
//! column. It carries no text, so it can be clearer than the chrome, and the
//! backdrop's sky shows through it. Soft fades join it to the column.
//!
//! The chrome, the page and the borders are all tinted a little toward the
//! theme's primary color, so that every theme has a mood of its own. The tint
//! is faint: the theme's three brand colors carry its hues, and surfaces that
//! are as colored as they are would drown them.

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{
    App, Div, Hsla, ParentElement as _, Rgba, Styled as _, Window, WindowAppearance, div,
    linear_color_stop, linear_gradient, px,
};

use crate::backdrop;
use crate::contrast::{self, BLACK, PANE_FLOOR, Rgb, WHITE};

/// Chrome opacity when the backdrop is known to be on the theme's side: a
/// dark theme on a dark system, or light on light.
pub(crate) const CHROME_MATCHED_FLOOR: f32 = 0.62;
/// Chrome opacity when it is not: a dark theme over a light backdrop needs
/// far denser glass to keep muted text readable.
const CHROME_FLOOR: f32 = 0.86;

/// Margin opacity when the backdrop is on the theme's side. A margin holds no
/// text, so this is a matter of taste, not of contrast; chosen by eye on
/// 2026-11-24. On the other side the
/// margin takes the chrome's opacity: a dark page on clear light glass looks
/// like a hole.
pub(crate) const MARGIN_MATCHED: f32 = 0.35;
/// Width of the fade from a margin to the reading column, in design units.
pub(crate) const FADE: f32 = 96.0;
/// Space at the column's full opacity on each side of its text, so that the
/// fade never runs under a letter, in design units.
const COLUMN_INSET: f32 = 12.0;
/// The narrowest a margin may be, fade not counted, in design units. A
/// narrower one is left at the column's opacity.
const MIN_MARGIN: f32 = 24.0;

/// Share of the primary color mixed into every chrome surface.
pub(crate) const CHROME_TINT: f32 = 0.06;
/// Share of the primary color mixed into the reading pane, as much as the
/// chrome so that the page and its frame read as one surface.
pub const PANE_TINT: f32 = 0.06;
/// How far borders are pulled toward the primary color.
const BORDER_PULL: f32 = 0.25;

/// Mica is the wallpaper, blurred and pulled toward the system mode's base
/// color. These are the lightest dark Mica and the darkest light Mica.
const DARK_MICA_LIMIT: Rgb = [0.25; 3];
const LIGHT_MICA_LIMIT: Rgb = [0.78; 3];
/// Any backdrop at all.
const UNKNOWN: [Rgb; 2] = [BLACK, WHITE];

/// What can be behind the chrome when the system is in the theme's mode:
/// Mica, with the drifting backdrop over it. The drifting shapes are theme
/// colors at `backdrop::STRENGTH`; taking them as pure white on a dark system
/// and pure black on a light one bounds any color they can be.
pub(crate) fn matched_backdrop(system_dark: bool) -> [Rgb; 2] {
    if system_dark {
        let lightest = contrast::over(WHITE, backdrop::STRENGTH, DARK_MICA_LIMIT);
        [BLACK, lightest]
    } else {
        let darkest = contrast::over(BLACK, backdrop::STRENGTH, LIGHT_MICA_LIMIT);
        [darkest, WHITE]
    }
}

/// Opacity of the two kinds of surface, from 0 (clear) to 1 (solid).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Levels {
    pub pane: f32,
    pub chrome: f32,
    /// The empty sides of a page, beside its reading column.
    pub margin: f32,
}

/// The levels of the frame being drawn, for components that are not handed
/// them.
impl gpui_kit::Global for Levels {}

impl Levels {
    /// Solid surfaces, for before the first frame.
    pub const OPAQUE: Self = Self {
        pane: 1.0,
        chrome: 1.0,
        margin: 1.0,
    };

    /// The levels for the active theme in this window.
    pub fn for_window(window: &Window, cx: &App) -> Self {
        let theme = cx.theme();
        let system_dark = matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        Self::compute(
            rgb(tint(theme.background, theme.primary, PANE_TINT)),
            rgb(tint(theme.background, theme.primary, CHROME_TINT)),
            // Links on the page and section labels in the chrome are drawn
            // in the link color.
            &[
                rgb(theme.foreground),
                rgb(theme.muted_foreground),
                rgb(theme.link),
            ],
            theme.is_dark(),
            system_dark,
        )
    }

    /// Each surface is the theme's background with its tint mixed in.
    fn compute(
        pane_surface: Rgb,
        chrome_surface: Rgb,
        texts: &[Rgb],
        theme_dark: bool,
        system_dark: bool,
    ) -> Self {
        let pane = contrast::fit_alpha(pane_surface, texts, PANE_FLOOR, UNKNOWN);
        let matched = theme_dark == system_dark;
        let chrome = if matched {
            let behind = matched_backdrop(system_dark);
            contrast::fit_alpha(chrome_surface, texts, CHROME_MATCHED_FLOOR, behind)
        } else {
            contrast::fit_alpha(chrome_surface, texts, CHROME_FLOOR, UNKNOWN)
        };
        // Chrome is never denser than the pane it frames.
        let chrome = chrome.min(pane);
        Self {
            pane,
            chrome,
            margin: if matched {
                MARGIN_MATCHED.min(chrome)
            } else {
                chrome
            },
        }
    }

    /// A margin surface in `color`.
    pub fn margin(self, color: Hsla) -> Hsla {
        color.opacity(self.margin)
    }

    /// A pane surface in `color`.
    pub fn pane(self, color: Hsla) -> Hsla {
        color.opacity(self.pane)
    }

    /// A chrome surface in `color`.
    pub fn chrome(self, color: Hsla) -> Hsla {
        color.opacity(self.chrome)
    }
}

fn rgb(color: Hsla) -> Rgb {
    let color = color.to_rgb();
    [color.r, color.g, color.b]
}

/// `surface` with `share` of `primary` mixed in.
pub fn tint(surface: Hsla, primary: Hsla, share: f32) -> Hsla {
    surface.blend(primary.opacity(share))
}

/// A chrome surface: `surface` tinted toward the active theme's primary
/// color. Pass the result to [`Levels::chrome`].
pub fn tinted(surface: Hsla, cx: &App) -> Hsla {
    tint(surface, cx.theme().primary, CHROME_TINT)
}

/// The reading pane's surface: `surface` tinted toward the active theme's
/// primary color. Pass the result to [`Levels::pane`].
pub fn tinted_pane(surface: Hsla, cx: &App) -> Hsla {
    tint(surface, cx.theme().primary, PANE_TINT)
}

/// How wide each clear margin is beside a reading column whose text starts
/// `gutter` in from the pane's sides, fade not counted; `None` when there is
/// no room for one. In design units.
pub fn margin_width(gutter: f32) -> Option<f32> {
    let width = gutter - COLUMN_INSET - FADE;
    (width >= MIN_MARGIN).then_some(width)
}

/// The surface of a reading pane: clear margins `margin` wide (from
/// [`margin_width`]) on each side, soft fades, and the column at the pane's
/// opacity between them. Lay it under the page, filling the pane.
pub fn reading_surface(levels: Levels, margin: f32, cx: &App) -> Div {
    use crate::zoom::px;
    let color = tinted_pane(cx.theme().background, cx);
    let (side, column) = (levels.margin(color), levels.pane(color));
    // 90 degrees runs left to right, 270 right to left.
    let fade = |angle: f32| {
        linear_gradient(
            angle,
            linear_color_stop(side, 0.),
            linear_color_stop(column, 1.),
        )
    };
    let strip = || div().h_full().flex_none();
    div()
        .flex()
        .size_full()
        .child(strip().w(px(margin)).bg(side))
        .child(strip().w(px(FADE)).bg(fade(90.)))
        .child(div().h_full().flex_1().bg(column))
        .child(strip().w(px(FADE)).bg(fade(270.)))
        .child(strip().w(px(margin)).bg(side))
}

/// The theme's border, pulled toward its primary color.
pub fn tinted_border(cx: &App) -> Hsla {
    let theme = cx.theme();
    theme.border.blend(theme.primary.opacity(BORDER_PULL))
}

/// A one-pixel strip across the top of a surface that starts in the primary
/// color and fades out to the right. One pixel of the screen at every zoom,
/// as a border is.
#[allow(clippy::disallowed_methods)]
pub fn hairline(cx: &App) -> Div {
    let primary = cx.theme().primary;
    div().h(px(1.)).w_full().flex_shrink_0().bg(linear_gradient(
        90.,
        linear_color_stop(primary.opacity(0.75), 0.),
        linear_color_stop(primary.opacity(0.), 1.),
    ))
}

/// An opaque color to paint under every surface instead of the real backdrop,
/// from `CODEWIKI_BACKDROP=#RRGGBB`.
///
/// A window capture cannot see Mica, so this is how the glass is checked: with
/// a known color behind it, a capture shows exactly what each surface lets
/// through.
pub fn debug_backdrop() -> Option<Hsla> {
    let value = std::env::var("CODEWIKI_BACKDROP").ok()?;
    let [r, g, b] = contrast::parse_hex(value.trim())?;
    Some(Rgba { r, g, b, a: 1.0 }.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contrast::parse_hex;

    fn hex(value: &str) -> Rgb {
        parse_hex(value).unwrap()
    }

    // CodeWiki Violet: background, foreground, muted foreground, link,
    // primary.
    const VIOLET: [&str; 5] = ["#0E0D15", "#E9E8EE", "#AAA8B6", "#E886C9", "#7C4DEE"];
    const VIOLET_LIGHT: [&str; 5] = ["#FAFAFC", "#1A1A1E", "#4F4D5A", "#910F74", "#6D3FE0"];

    fn levels(theme: [&str; 5], theme_dark: bool, system_dark: bool) -> Levels {
        let background = hex(theme[0]);
        let primary = hex(theme[4]);
        Levels::compute(
            contrast::over(primary, PANE_TINT, background),
            contrast::over(primary, CHROME_TINT, background),
            &[hex(theme[1]), hex(theme[2]), hex(theme[3])],
            theme_dark,
            system_dark,
        )
    }

    #[test]
    fn a_tinted_surface_moves_toward_the_primary_color_and_stays_opaque() {
        let surface = gpui_kit::hsla(0.0, 0.0, 0.06, 1.0);
        let primary = gpui_kit::hsla(0.72, 0.8, 0.6, 1.0);
        let share = CHROME_TINT;
        let tinted = tint(surface, primary, share);
        assert_eq!(tinted.a, 1.0);
        let (from, toward, got) = (rgb(surface), rgb(primary), rgb(tinted));
        for channel in 0..3 {
            let expected = contrast::over(toward, share, from)[channel];
            assert!((got[channel] - expected).abs() < 1e-3, "{got:?}");
        }
        assert!(tinted.s > surface.s);
    }

    #[test]
    fn a_codewiki_theme_gets_the_clearest_glass_on_a_matching_system() {
        let dark = levels(VIOLET, true, true);
        assert_eq!(dark.pane, PANE_FLOOR);
        assert_eq!(dark.chrome, CHROME_MATCHED_FLOOR);
        let light = levels(VIOLET_LIGHT, false, false);
        assert_eq!(light.pane, PANE_FLOOR);
        assert_eq!(light.chrome, CHROME_MATCHED_FLOOR);
    }

    /// A bundled theme whose text were too weak would get denser glass than
    /// the others, and would look like another app.
    #[test]
    fn every_bundled_theme_gets_the_clearest_glass_on_a_matching_system() {
        for content in crate::themes::BUNDLED {
            let set: serde_json::Value = serde_json::from_str(content).unwrap();
            for theme in set["themes"].as_array().unwrap() {
                let color = |key: &str| hex(theme["colors"][key].as_str().unwrap());
                let dark = theme["mode"] == "dark";
                let (background, primary) = (color("background"), color("primary.background"));
                let levels = Levels::compute(
                    contrast::over(primary, PANE_TINT, background),
                    contrast::over(primary, CHROME_TINT, background),
                    &[
                        color("foreground"),
                        color("muted.foreground"),
                        color("link"),
                    ],
                    dark,
                    dark,
                );
                let expected = Levels {
                    pane: PANE_FLOOR,
                    chrome: CHROME_MATCHED_FLOOR,
                    margin: MARGIN_MATCHED,
                };
                assert_eq!(levels, expected, "{}", theme["name"]);
            }
        }
    }

    #[test]
    fn the_tinted_page_is_still_glass_and_the_chrome_no_denser() {
        for (theme, dark) in [(VIOLET, true), (VIOLET_LIGHT, false)] {
            let background = hex(theme[0]);
            let primary = hex(theme[4]);
            let levels = Levels::compute(
                contrast::over(primary, PANE_TINT, background),
                contrast::over(primary, CHROME_TINT, background),
                &[hex(theme[1]), hex(theme[2]), hex(theme[3])],
                dark,
                dark,
            );
            assert!(levels.pane < 1.0, "{levels:?}");
            assert!(levels.chrome <= levels.pane, "{levels:?}");
        }
    }

    #[test]
    fn chrome_is_denser_when_the_system_mode_is_the_other_one() {
        let dark_on_light = levels(VIOLET, true, false);
        assert!(dark_on_light.chrome >= CHROME_FLOOR, "{dark_on_light:?}");
        let light_on_dark = levels(VIOLET_LIGHT, false, true);
        assert!(light_on_dark.chrome >= CHROME_FLOOR, "{light_on_dark:?}");
    }

    #[test]
    fn chrome_is_never_denser_than_the_pane() {
        for (theme, dark) in [(VIOLET, true), (VIOLET_LIGHT, false)] {
            for system_dark in [true, false] {
                let levels = levels(theme, dark, system_dark);
                assert!(levels.chrome <= levels.pane, "{levels:?}");
            }
        }
    }

    #[test]
    fn a_margin_is_the_clearest_glass_and_follows_the_chrome_on_the_other_side() {
        for (theme, dark) in [(VIOLET, true), (VIOLET_LIGHT, false)] {
            let matched = levels(theme, dark, dark);
            assert_eq!(matched.margin, MARGIN_MATCHED);
            assert!(matched.margin < matched.chrome, "{matched:?}");
            let other = levels(theme, dark, !dark);
            assert_eq!(other.margin, other.chrome, "{other:?}");
        }
    }

    #[test]
    fn a_margin_needs_room_beside_its_fade() {
        // A wide window: the margin is what the fade and the inset leave.
        assert_eq!(margin_width(300.0), Some(300.0 - COLUMN_INSET - FADE));
        assert_eq!(
            margin_width(COLUMN_INSET + FADE + MIN_MARGIN),
            Some(MIN_MARGIN)
        );
        // Too narrow, or the least gutter a page keeps: no margin at all.
        assert_eq!(margin_width(COLUMN_INSET + FADE + MIN_MARGIN - 1.0), None);
        assert_eq!(margin_width(24.0), None);
    }

    #[test]
    fn a_low_contrast_theme_gets_denser_glass_than_a_strong_one() {
        // Muted text at about 3:1 on its own background, as several of the
        // older bundled themes have.
        let weak = Levels::compute(
            hex("#1a1b26"),
            hex("#1a1b26"),
            &[hex("#c0caf5"), hex("#565f89")],
            true,
            false,
        );
        let strong = levels(VIOLET, true, false);
        assert!(weak.pane > strong.pane, "{weak:?} vs {strong:?}");
        assert!(weak.pane <= 1.0);
    }
}
