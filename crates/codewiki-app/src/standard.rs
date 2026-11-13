//! Standard levels for every theme.
//!
//! The bundled themes come from many authors and sit at very different
//! levels: some are near black with a dark accent, some sit on a light gray
//! with a pastel one. Under the app's tinted glass that spread shows as themes
//! that are far too dark or too bright. Every theme other than CodeWiki's own
//! is therefore redrawn on the CodeWiki set's levels: each color role takes
//! the lightness and saturation the CodeWiki themes of the same mode give it,
//! in the theme's own hue. Code syntax colors stay as the author made them.
//!
//! The levels are read from `codewiki.json` itself, so they follow any change
//! to the CodeWiki set. Text colors are then nudged until they keep the
//! contrast `glass` and the theme tests ask of the CodeWiki themes.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::sync::LazyLock;

use gpui_kit::component::{ThemeConfig, ThemeConfigColors};
use serde_json::{Map, Value};

use crate::contrast::{self, AA, BLACK, PANE_FLOOR, Rgb, WHITE};
use crate::glass;
use crate::themes::CODEWIKI;

/// What a color key is for, which decides the hue it is drawn in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// Surfaces, borders and plain text, in the theme's accent hue.
    Neutral,
    /// The accent itself and what is drawn in it.
    Accent,
    /// A color with a meaning of its own (status, palette), in its own hue.
    Own,
    /// Text on the named fill: white or the background, whichever reads
    /// better on it.
    OnFill(&'static str),
}

/// Every color key the CodeWiki themes set, with its role.
const ROLES: [(&str, Role); 59] = [
    ("background", Role::Neutral),
    ("foreground", Role::Neutral),
    ("border", Role::Neutral),
    ("window.border", Role::Neutral),
    ("accent.background", Role::Neutral),
    ("accent.foreground", Role::Neutral),
    ("list.even.background", Role::Neutral),
    ("list.hover.background", Role::Neutral),
    ("muted.background", Role::Neutral),
    ("muted.foreground", Role::Neutral),
    ("popover.background", Role::Neutral),
    ("popover.foreground", Role::Neutral),
    ("secondary.background", Role::Neutral),
    ("secondary.foreground", Role::Neutral),
    ("secondary.hover.background", Role::Neutral),
    ("secondary.active.background", Role::Neutral),
    ("scrollbar.background", Role::Neutral),
    ("scrollbar.thumb.background", Role::Neutral),
    ("sidebar.background", Role::Neutral),
    ("sidebar.foreground", Role::Neutral),
    ("sidebar.border", Role::Neutral),
    ("sidebar.accent.foreground", Role::Neutral),
    ("tab.foreground", Role::Neutral),
    ("tab.active.foreground", Role::Neutral),
    ("tab_bar.background", Role::Neutral),
    ("title_bar.background", Role::Neutral),
    ("title_bar.border", Role::Neutral),
    ("chart.grid", Role::Neutral),
    ("primary.background", Role::Accent),
    ("ring", Role::Accent),
    ("link", Role::Accent),
    ("link.hover", Role::Accent),
    ("link.active", Role::Accent),
    ("list.active.background", Role::Accent),
    ("list.active.border", Role::Accent),
    ("selection.background", Role::Accent),
    ("sidebar.accent.background", Role::Accent),
    ("sidebar.primary.background", Role::Accent),
    ("chart.1", Role::Accent),
    ("success.background", Role::Own),
    ("warning.background", Role::Own),
    ("danger.background", Role::Own),
    ("info.background", Role::Own),
    ("chart.2", Role::Own),
    ("chart.3", Role::Own),
    ("chart.4", Role::Own),
    ("chart.5", Role::Own),
    ("base.red", Role::Own),
    ("base.green", Role::Own),
    ("base.yellow", Role::Own),
    ("base.blue", Role::Own),
    ("base.magenta", Role::Own),
    ("base.cyan", Role::Own),
    ("primary.foreground", Role::OnFill("primary.background")),
    (
        "sidebar.primary.foreground",
        Role::OnFill("sidebar.primary.background"),
    ),
    ("success.foreground", Role::OnFill("success.background")),
    ("warning.foreground", Role::OnFill("warning.background")),
    ("danger.foreground", Role::OnFill("danger.background")),
    ("info.foreground", Role::OnFill("info.background")),
];

/// Colors that carry text on the page, held to AA as the CodeWiki themes are
/// (`themes.rs`). The first three are also the texts `glass` fits the
/// chrome's opacity to.
const TEXT_KEYS: [&str; 9] = [
    "foreground",
    "muted.foreground",
    "link",
    "success.background",
    "warning.background",
    "danger.background",
    "info.background",
    "base.magenta",
    "base.cyan",
];
const GLASS_TEXTS: usize = 3;
/// Colors drawn as icons, held to WCAG's 3:1 for graphics.
const ICON_KEYS: [&str; 4] = ["base.red", "base.green", "base.yellow", "base.blue"];
const ICON_CONTRAST: f32 = 3.0;

/// Keys that take another key's final color, as they do in the CodeWiki set.
const FOLLOWERS: [(&str, &str); 3] = [
    ("link.hover", "link"),
    ("link.active", "link"),
    ("tab.foreground", "muted.foreground"),
];

/// A hue is only taken from a color with at least this much chroma; below it
/// the color reads as gray and its hue is noise.
const MIN_CHROMA: f64 = 0.03;
/// How close to the edge of sRGB a color may go: neutrals stay well inside so
/// no channel is pinned, accents and status colors may go further.
const NEUTRAL_GAMUT: f64 = 0.8;
const COLOR_GAMUT: f64 = 0.95;
/// One step of the contrast nudge, in OKLCH lightness.
const NUDGE: f64 = 0.005;

/// One color key's level in the CodeWiki set: mean OKLCH lightness and
/// chroma, mean hue (used when a theme has no hue of its own for the key),
/// and mean alpha.
#[derive(Clone, Copy, Debug)]
struct Level {
    l: f64,
    c: f64,
    h: f64,
    alpha: f32,
}

struct Reference {
    dark: HashMap<String, Level>,
    light: HashMap<String, Level>,
    /// The CodeWiki themes, which are the reference and stay as they are.
    names: HashSet<String>,
}

static REFERENCE: LazyLock<Reference> = LazyLock::new(|| Reference::from_json(CODEWIKI));

impl Reference {
    fn from_json(content: &str) -> Self {
        #[derive(Default)]
        struct Sum {
            l: f64,
            c: f64,
            cos: f64,
            sin: f64,
            alpha: f32,
            count: u32,
        }
        let set: Value = serde_json::from_str(content).expect("codewiki.json is valid JSON");
        let mut sums: [HashMap<String, Sum>; 2] = Default::default();
        let mut names = HashSet::new();
        for theme in set["themes"].as_array().into_iter().flatten() {
            names.insert(theme["name"].as_str().unwrap_or_default().to_string());
            let mode = usize::from(theme["mode"] == "dark");
            for (key, value) in theme["colors"].as_object().into_iter().flatten() {
                let Some((rgb, alpha)) = value.as_str().and_then(parse_rgba) else {
                    continue;
                };
                let [l, c, h] = to_oklch(rgb);
                let sum = sums[mode].entry(key.clone()).or_default();
                sum.l += l;
                sum.c += c;
                sum.cos += c * h.to_radians().cos();
                sum.sin += c * h.to_radians().sin();
                sum.alpha += alpha;
                sum.count += 1;
            }
        }
        let [light, dark] = sums.map(|sums| {
            sums.into_iter()
                .map(|(key, sum)| {
                    let n = f64::from(sum.count);
                    let level = Level {
                        l: sum.l / n,
                        c: sum.c / n,
                        h: sum.sin.atan2(sum.cos).to_degrees().rem_euclid(360.0),
                        alpha: sum.alpha / sum.count as f32,
                    };
                    (key, level)
                })
                .collect()
        });
        Self { dark, light, names }
    }
}

/// `config` on the CodeWiki set's levels. CodeWiki's own themes, and any
/// theme whose colors cannot be read, are returned unchanged.
pub fn standardize(config: &ThemeConfig) -> ThemeConfig {
    let reference = &*REFERENCE;
    if reference.names.contains(config.name.as_ref()) {
        return config.clone();
    }
    let Ok(Value::Object(mut colors)) = serde_json::to_value(&config.colors) else {
        return config.clone();
    };
    let dark = config.mode.is_dark();
    let levels = if dark {
        &reference.dark
    } else {
        &reference.light
    };
    let new = standard_colors(&colors, levels, dark);
    for (key, (rgb, alpha)) in &new {
        colors.insert((*key).to_string(), Value::String(hex(*rgb, *alpha)));
    }
    let mut standardized = config.clone();
    match serde_json::from_value::<ThemeConfigColors>(Value::Object(colors)) {
        Ok(colors) => standardized.colors = colors,
        Err(_) => return config.clone(),
    }
    if let Some(highlight) =
        editor_colors(config, &new).and_then(|value| serde_json::from_value(value).ok())
    {
        standardized.highlight = Some(highlight);
    }
    standardized
}

/// The standard color of every role key, from the theme's own `colors`.
fn standard_colors(
    colors: &Map<String, Value>,
    levels: &HashMap<String, Level>,
    dark: bool,
) -> HashMap<&'static str, (Rgb, f32)> {
    let own = |key: &str| {
        colors
            .get(key)
            .and_then(Value::as_str)
            .and_then(parse_rgba)
            .map(|(rgb, _)| to_oklch(rgb))
            .filter(|lch| lch[1] >= MIN_CHROMA)
    };
    // The theme's accent hue; a theme with none stays gray.
    let accent_hue = ["primary.background", "link", "ring", "info.background"]
        .into_iter()
        .find_map(own)
        .map(|lch| lch[2]);

    let mut out = HashMap::new();
    for (key, role) in ROLES {
        let Some(level) = levels.get(key) else {
            continue;
        };
        let (hue, gamut) = match role {
            Role::Neutral => (accent_hue, NEUTRAL_GAMUT),
            Role::Accent => (accent_hue, COLOR_GAMUT),
            Role::Own => (Some(own(key).map_or(level.h, |lch| lch[2])), COLOR_GAMUT),
            Role::OnFill(_) => continue,
        };
        let rgb = match hue {
            Some(h) => from_oklch(level.l, level.c, h, gamut),
            None => from_oklch(level.l, 0.0, 0.0, gamut),
        };
        out.insert(key, (rgb, level.alpha));
    }

    nudge_texts(&mut out, dark);
    for (follower, leader) in FOLLOWERS {
        if let Some(&(rgb, _)) = out.get(leader)
            && let Some(entry) = out.get_mut(follower)
        {
            entry.0 = rgb;
        }
    }
    let background = out.get("background").map_or(BLACK, |entry| entry.0);
    for (key, role) in ROLES {
        if let Role::OnFill(fill) = role
            && let Some(&(fill, _)) = out.get(fill)
        {
            let text = if contrast::ratio(WHITE, fill) >= contrast::ratio(background, fill) {
                WHITE
            } else {
                background
            };
            out.insert(key, (text, 1.0));
        }
    }
    out
}

/// Move each text and icon color's lightness, away from the page, until it
/// reads on every surface it is drawn on.
fn nudge_texts(out: &mut HashMap<&'static str, (Rgb, f32)>, dark: bool) {
    let color = |out: &HashMap<&str, (Rgb, f32)>, key| out.get(key).map(|entry| entry.0);
    let (Some(page), Some(card), Some(sidebar), Some(primary)) = (
        color(out, "background"),
        color(out, "secondary.background"),
        color(out, "sidebar.background"),
        color(out, "primary.background"),
    ) else {
        return;
    };
    let tinted_page = contrast::over(primary, glass::PANE_TINT, page);
    let chrome = contrast::over(primary, glass::CHROME_TINT, page);
    let behind_chrome = glass::matched_backdrop(dark);
    let any = [BLACK, WHITE];
    let worst = |text, surface| contrast::worst_ratio(text, surface, PANE_FLOOR, any);

    let reads_as_text = |text: Rgb, glass_text: bool| {
        [page, card, tinted_page]
            .into_iter()
            .all(|surface| worst(text, surface) >= AA)
            && (!glass_text
                || contrast::fit_alpha(chrome, &[text], glass::CHROME_MATCHED_FLOOR, behind_chrome)
                    <= glass::CHROME_MATCHED_FLOOR + 1e-4)
    };
    let reads_as_icon = |icon: Rgb| {
        [page, sidebar]
            .into_iter()
            .all(|surface| worst(icon, surface) >= ICON_CONTRAST)
    };

    let checks = TEXT_KEYS
        .iter()
        .enumerate()
        .map(|(index, key)| (*key, Some(index < GLASS_TEXTS)))
        .chain(ICON_KEYS.iter().map(|key| (*key, None)));
    for (key, glass_text) in checks {
        let Some(entry) = out.get_mut(key) else {
            continue;
        };
        let reads = |rgb: Rgb| match glass_text {
            Some(glass_text) => reads_as_text(rgb, glass_text),
            None => reads_as_icon(rgb),
        };
        let [mut l, c, h] = to_oklch(entry.0);
        let mut rgb = entry.0;
        while !reads(rgb) && (0.0..=1.0).contains(&l) {
            l += if dark { NUDGE } else { -NUDGE };
            rgb = from_oklch(l.clamp(0.0, 1.0), c, h, COLOR_GAMUT);
        }
        entry.0 = rgb;
    }
}

/// The theme's highlight section with its editor colors set from the
/// standard ones, or `None` when the theme has no highlight section.
fn editor_colors(config: &ThemeConfig, new: &HashMap<&'static str, (Rgb, f32)>) -> Option<Value> {
    let mut highlight = serde_json::to_value(config.highlight.as_ref()?).ok()?;
    let object = highlight.as_object_mut()?;
    for (editor_key, key) in [
        ("editor.background", "background"),
        ("editor.foreground", "foreground"),
        ("editor.active_line.background", "secondary.background"),
        ("editor.line_number", "muted.foreground"),
        ("editor.active_line_number", "foreground"),
    ] {
        let (rgb, _) = new.get(key)?;
        object.insert(editor_key.to_string(), Value::String(hex(*rgb, 1.0)));
    }
    Some(highlight)
}

/// `#RRGGBB` or `#RRGGBBAA` as a color and its alpha.
fn parse_rgba(value: &str) -> Option<(Rgb, f32)> {
    let rgb = contrast::parse_hex(value)?;
    let alpha = match value.len() {
        9 => f32::from(u8::from_str_radix(value.get(7..9)?, 16).ok()?) / 255.0,
        _ => 1.0,
    };
    Some((rgb, alpha))
}

fn hex(rgb: Rgb, alpha: f32) -> String {
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    let mut out = format!(
        "#{:02X}{:02X}{:02X}",
        byte(rgb[0]),
        byte(rgb[1]),
        byte(rgb[2])
    );
    if alpha < 1.0 {
        let _ = write!(out, "{:02X}", byte(alpha));
    }
    out
}

fn to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn to_gamma(c: f64) -> f64 {
    if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// sRGB to OKLCH: lightness 0–1, chroma, hue in degrees. Björn Ottosson's
/// OKLab matrices.
fn to_oklch(rgb: Rgb) -> [f64; 3] {
    let [r, g, b] = rgb.map(|c| to_linear(f64::from(c)));
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    let lightness = 0.210_454_255_3 * l + 0.793_617_785 * m - 0.004_072_046_8 * s;
    let a = 1.977_998_495_1 * l - 2.428_592_205 * m + 0.450_593_709_9 * s;
    let b = 0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766 * s;
    [
        lightness,
        a.hypot(b),
        b.atan2(a).to_degrees().rem_euclid(360.0),
    ]
}

/// OKLCH to linear sRGB, which may fall outside 0–1.
fn linear_from_oklch(l: f64, c: f64, h: f64) -> [f64; 3] {
    let (a, b) = (c * h.to_radians().cos(), c * h.to_radians().sin());
    let l_ = (l + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let m_ = (l - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let s_ = (l - 0.089_484_177_5 * a - 1.291_485_548 * b).powi(3);
    [
        4.076_741_662_1 * l_ - 3.307_711_591_3 * m_ + 0.230_969_929_2 * s_,
        -1.268_438_004_6 * l_ + 2.609_757_401_1 * m_ - 0.341_319_396_5 * s_,
        -0.004_196_086_3 * l_ - 0.703_418_614_7 * m_ + 1.707_614_701 * s_,
    ]
}

/// The most chroma a color of lightness `l` and hue `h` can have in sRGB.
fn max_chroma(l: f64, h: f64) -> f64 {
    let in_gamut = |c| {
        linear_from_oklch(l, c, h)
            .iter()
            .all(|v| (-1e-6..=1.0 + 1e-6).contains(v))
    };
    let (mut low, mut high) = (0.0, 0.4);
    for _ in 0..30 {
        let mid = (low + high) / 2.0;
        if in_gamut(mid) {
            low = mid;
        } else {
            high = mid;
        }
    }
    low
}

/// The sRGB color at OKLCH `l`, `c`, `h`, with the chroma held to `gamut`
/// of what sRGB allows there. Channels are rounded to 8 bits, so contrast is
/// checked on exactly the color that is written out.
fn from_oklch(l: f64, c: f64, h: f64, gamut: f64) -> Rgb {
    let c = c.min(gamut * max_chroma(l, h));
    linear_from_oklch(l, c, h)
        .map(|v| ((to_gamma(v.clamp(0.0, 1.0)) * 255.0).round() / 255.0) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes::BUNDLED;

    fn configs(content: &str) -> Vec<ThemeConfig> {
        let set: Value = serde_json::from_str(content).unwrap();
        set["themes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|theme| serde_json::from_value(theme.clone()).unwrap())
            .collect()
    }

    fn colors(config: &ThemeConfig) -> Map<String, Value> {
        match serde_json::to_value(&config.colors).unwrap() {
            Value::Object(colors) => colors,
            _ => unreachable!(),
        }
    }

    fn color(config: &ThemeConfig, key: &str) -> Rgb {
        let colors = colors(config);
        let value = colors[key].as_str().unwrap_or_else(|| {
            panic!("{}: {key} is not set", config.name);
        });
        parse_rgba(value).unwrap().0
    }

    #[test]
    fn every_codewiki_color_key_has_a_role() {
        let roles: HashSet<&str> = ROLES.iter().map(|(key, _)| *key).collect();
        assert_eq!(roles.len(), ROLES.len(), "a key is listed twice");
        for config in configs(CODEWIKI) {
            let keys: HashSet<String> = colors(&config)
                .into_iter()
                .filter(|(_, value)| !value.is_null())
                .map(|(key, _)| key)
                .collect();
            let keys: HashSet<&str> = keys.iter().map(String::as_str).collect();
            assert_eq!(keys, roles, "{}", config.name);
        }
    }

    #[test]
    fn oklch_round_trips_srgb() {
        for hex_value in [
            "#0E0922", "#7C4DEE", "#FFFFFF", "#000000", "#F59F2E", "#2DD4A7",
        ] {
            let rgb = contrast::parse_hex(hex_value).unwrap();
            let [l, c, h] = to_oklch(rgb);
            let back = from_oklch(l, c, h, 1.0);
            for channel in 0..3 {
                assert!(
                    (back[channel] - rgb[channel]).abs() < 1.0 / 255.0,
                    "{hex_value}"
                );
            }
        }
    }

    #[test]
    fn the_codewiki_themes_are_left_as_they_are() {
        for config in configs(CODEWIKI) {
            assert_eq!(
                colors(&standardize(&config)),
                colors(&config),
                "{}",
                config.name
            );
        }
    }

    /// Every bundled theme, standardized, reads as the CodeWiki themes are
    /// required to: text at AA on the page, a card and the tinted page; icons
    /// at 3:1; the page itself at the CodeWiki set's lightness.
    #[test]
    fn every_bundled_theme_meets_the_codewiki_rules_once_standardized() {
        let mut failures = Vec::new();
        for content in BUNDLED {
            for config in configs(content) {
                let theme = standardize(&config);
                let name = theme.name.to_string();
                let page = color(&theme, "background");
                let card = color(&theme, "secondary.background");
                let sidebar = color(&theme, "sidebar.background");
                let tinted =
                    contrast::over(color(&theme, "primary.background"), glass::PANE_TINT, page);
                let worst = |text, surface| {
                    contrast::worst_ratio(text, surface, PANE_FLOOR, [BLACK, WHITE])
                };
                for key in TEXT_KEYS {
                    for (surface, fill) in [("page", page), ("card", card), ("tinted", tinted)] {
                        let ratio = worst(color(&theme, key), fill);
                        if ratio < AA {
                            failures.push(format!("{name}: {key} on {surface} is {ratio:.2}"));
                        }
                    }
                }
                for key in ICON_KEYS.into_iter().chain(["link"]) {
                    for fill in [page, sidebar] {
                        let ratio = worst(color(&theme, key), fill);
                        if ratio < ICON_CONTRAST {
                            failures.push(format!("{name}: icon {key} is {ratio:.2}"));
                        }
                    }
                }
                // The CodeWiki themes are the reference; each sits near, not
                // on, their mean.
                if !REFERENCE.names.contains(&name) {
                    let levels = if theme.mode.is_dark() {
                        &REFERENCE.dark
                    } else {
                        &REFERENCE.light
                    };
                    let reference = levels["background"].l;
                    let lightness = to_oklch(page)[0];
                    if (lightness - reference).abs() > 0.01 {
                        failures.push(format!("{name}: page lightness {lightness:.3}"));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    #[test]
    fn fahrenheit_keeps_its_red_and_gains_readable_links() {
        let fahrenheit = configs(include_str!("../themes/fahrenheit.json"))
            .into_iter()
            .find(|config| config.name.as_ref() == "Fahrenheit")
            .unwrap();
        let theme = standardize(&fahrenheit);
        let link = color(&theme, "link");
        let page = color(&theme, "background");
        assert!(contrast::ratio(link, page) >= AA);
        let original = to_oklch(contrast::parse_hex("#720202").unwrap())[2];
        let hue = to_oklch(color(&theme, "primary.background"))[2];
        let apart = (hue - original).abs().min(360.0 - (hue - original).abs());
        assert!(apart < 20.0, "accent hue moved {apart:.0} degrees");
    }
}
