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
//! Chrome is also tinted toward the theme's primary color, so that every
//! theme reads in its own hue rather than in plain black or white.

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{
    App, Div, Hsla, Rgba, Styled as _, Window, WindowAppearance, div, linear_color_stop,
    linear_gradient, px,
};

use crate::backdrop;
use crate::contrast::{self, BLACK, PANE_FLOOR, Rgb, WHITE};

/// Chrome opacity when the backdrop is known to be on the theme's side: a
/// dark theme on a dark system, or light on light.
const CHROME_MATCHED_FLOOR: f32 = 0.62;
/// Chrome opacity when it is not: a dark theme over a light backdrop needs
/// far denser glass to keep muted text readable.
const CHROME_FLOOR: f32 = 0.86;

/// Share of the primary color mixed into every chrome surface. TradingAgent
/// tints its cards by this much (`components.ts`, `MuiCard`); see
/// `surfaces::CARD_TINT`.
const CHROME_TINT: f32 = 0.08;

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
fn matched_backdrop(system_dark: bool) -> [Rgb; 2] {
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
}

/// The levels of the frame being drawn, for components that are not handed
/// them.
impl gpui_kit::Global for Levels {}

impl Levels {
    /// Solid surfaces, for before the first frame.
    pub const OPAQUE: Self = Self {
        pane: 1.0,
        chrome: 1.0,
    };

    /// The levels for the active theme in this window.
    pub fn for_window(window: &Window, cx: &App) -> Self {
        let theme = cx.theme();
        let system_dark = matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        Self::compute(
            rgb(theme.background),
            rgb(tint(theme.background, theme.primary)),
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

    /// `chrome_surface` is the pane's surface with the primary tint mixed in.
    fn compute(
        pane_surface: Rgb,
        chrome_surface: Rgb,
        texts: &[Rgb],
        theme_dark: bool,
        system_dark: bool,
    ) -> Self {
        let pane = contrast::fit_alpha(pane_surface, texts, PANE_FLOOR, UNKNOWN);
        let chrome = if theme_dark == system_dark {
            let behind = matched_backdrop(system_dark);
            contrast::fit_alpha(chrome_surface, texts, CHROME_MATCHED_FLOOR, behind)
        } else {
            contrast::fit_alpha(chrome_surface, texts, CHROME_FLOOR, UNKNOWN)
        };
        // Chrome is never denser than the pane it frames.
        Self {
            pane,
            chrome: chrome.min(pane),
        }
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

/// `surface` with `primary` mixed in by [`CHROME_TINT`].
pub fn tint(surface: Hsla, primary: Hsla) -> Hsla {
    surface.blend(primary.opacity(CHROME_TINT))
}

/// A chrome surface: `surface` tinted toward the active theme's primary
/// color. Pass the result to [`Levels::chrome`].
pub fn tinted(surface: Hsla, cx: &App) -> Hsla {
    tint(surface, cx.theme().primary)
}

/// The theme's border, pulled a little toward its primary color.
pub fn tinted_border(cx: &App) -> Hsla {
    let theme = cx.theme();
    theme.border.blend(theme.primary.opacity(0.18))
}

/// A one-pixel strip across the top of a surface that starts in the primary
/// color and fades out to the right.
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
    const VIOLET: [&str; 5] = ["#0E0922", "#E9E7F7", "#A29ACE", "#AA8EFD", "#7C4DEE"];
    const VIOLET_LIGHT: [&str; 5] = ["#FAF9FF", "#1A1630", "#554E79", "#5B2FD1", "#6D3FE0"];

    fn levels(theme: [&str; 5], theme_dark: bool, system_dark: bool) -> Levels {
        let background = hex(theme[0]);
        let chrome = contrast::over(hex(theme[4]), CHROME_TINT, background);
        Levels::compute(
            background,
            chrome,
            &[hex(theme[1]), hex(theme[2]), hex(theme[3])],
            theme_dark,
            system_dark,
        )
    }

    #[test]
    fn a_tinted_surface_moves_toward_the_primary_color_and_stays_opaque() {
        let surface = gpui_kit::hsla(0.0, 0.0, 0.06, 1.0);
        let primary = gpui_kit::hsla(0.72, 0.8, 0.6, 1.0);
        let tinted = tint(surface, primary);
        assert_eq!(tinted.a, 1.0);
        let (from, toward, got) = (rgb(surface), rgb(primary), rgb(tinted));
        for channel in 0..3 {
            let expected = contrast::over(toward, CHROME_TINT, from)[channel];
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
