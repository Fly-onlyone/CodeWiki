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
//! A CodeWiki theme has three brand hues: primary, secondary and tertiary
//! (`themes::Brand`). A foreign theme seldom names more than one, so the
//! other two are found among the colors it does name, or set beside the
//! primary hue on the color wheel.
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
    /// Surfaces, borders and plain text, in the theme's primary hue.
    Neutral,
    /// The primary color and what is drawn in it: buttons, the active tab,
    /// the selection, the focus ring.
    Primary,
    /// The secondary color: links and what is drawn as one.
    Secondary,
    /// The tertiary color: marks and labels that stand apart from both.
    Tertiary,
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
    ("primary.background", Role::Primary),
    ("ring", Role::Primary),
    ("link", Role::Secondary),
    ("link.hover", Role::Secondary),
    ("link.active", Role::Secondary),
    ("list.active.background", Role::Primary),
    ("list.active.border", Role::Primary),
    ("selection.background", Role::Primary),
    ("sidebar.accent.background", Role::Primary),
    ("sidebar.primary.background", Role::Tertiary),
    ("chart.1", Role::Primary),
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
const TEXT_KEYS: [&str; 10] = [
    "foreground",
    "muted.foreground",
    "link",
    "sidebar.primary.background",
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
pub(crate) const MIN_CHROMA: f64 = 0.03;
/// How far, in degrees of hue, a secondary color sits from the primary one:
/// near enough to go with it, far enough to be told from it.
pub(crate) const SECONDARY_APART: (f64, f64) = (25.0, 90.0);
/// Where a secondary hue is set when the theme names none.
const SECONDARY_AIM: f64 = 40.0;
/// The least a tertiary color sits from the primary one.
pub(crate) const TERTIARY_APART: f64 = 80.0;
/// Where a tertiary hue may be set when the theme names none, from the
/// primary hue: far round the wheel on either side or, when the secondary
/// hue is already there, a quarter turn away.
const TERTIARY_AIMS: [f64; 4] = [150.0, -150.0, 90.0, -90.0];
/// The least the secondary and tertiary colors sit from each other.
pub(crate) const BRAND_APART: f64 = 60.0;
/// Colors a foreign theme may name, whose hues can serve as its secondary or
/// tertiary one.
const SPARE_HUES: [&str; 7] = [
    "base.red",
    "base.green",
    "base.yellow",
    "base.blue",
    "base.magenta",
    "base.cyan",
    "info.background",
];
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

/// The distance between two hues, in degrees, the short way round.
pub(crate) fn hue_apart(a: f64, b: f64) -> f64 {
    let apart = (a - b).rem_euclid(360.0);
    apart.min(360.0 - apart)
}

/// The hues of a theme's primary, secondary and tertiary colors, given the
/// chromatic colors it names. All are `None` for a theme with no hue at all,
/// which stays gray.
///
/// The secondary hue is the theme's link, when that is not its primary color
/// again; else whichever spare hue sits nearest to `SECONDARY_AIM` from the
/// primary; else that aim itself. The tertiary hue is the theme's own, when
/// it is far enough away; else the spare hue farthest from the other two;
/// else the one of `TERTIARY_AIMS` farthest from the other two.
fn brand_hues(own: impl Fn(&str) -> Option<[f64; 3]>) -> [Option<f64>; 3] {
    let hue = |key: &str| own(key).map(|lch| lch[2]);
    let Some(primary) = ["primary.background", "link", "ring", "info.background"]
        .into_iter()
        .find_map(hue)
    else {
        return [None; 3];
    };
    let spare: Vec<f64> = SPARE_HUES.into_iter().filter_map(hue).collect();
    let by = |a: f64, b: f64| a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal);

    let secondary = hue("link")
        .filter(|link| hue_apart(*link, primary) >= SECONDARY_APART.0)
        .or_else(|| {
            spare
                .iter()
                .copied()
                .filter(|spare| {
                    let apart = hue_apart(*spare, primary);
                    (SECONDARY_APART.0..=SECONDARY_APART.1).contains(&apart)
                })
                .min_by(|a, b| {
                    let off = |hue: f64| (hue_apart(hue, primary) - SECONDARY_AIM).abs();
                    by(off(*a), off(*b))
                })
        })
        .unwrap_or((primary + SECONDARY_AIM).rem_euclid(360.0));

    let clear = |hue: f64| hue_apart(hue, primary).min(hue_apart(hue, secondary));
    let tertiary = hue("sidebar.primary.background")
        .filter(|own| hue_apart(*own, primary) >= TERTIARY_APART)
        .or_else(|| {
            spare
                .iter()
                .copied()
                .filter(|spare| {
                    hue_apart(*spare, primary) >= TERTIARY_APART
                        && hue_apart(*spare, secondary) >= BRAND_APART
                })
                .max_by(|a, b| by(clear(*a), clear(*b)))
        })
        .unwrap_or_else(|| {
            let aims = TERTIARY_AIMS.map(|turn| (primary + turn).rem_euclid(360.0));
            aims.into_iter().fold(
                aims[0],
                |best, aim| if clear(aim) > clear(best) { aim } else { best },
            )
        });
    [Some(primary), Some(secondary), Some(tertiary)]
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
    let [primary_hue, secondary_hue, tertiary_hue] = brand_hues(own);

    let mut out = HashMap::new();
    for (key, role) in ROLES {
        let Some(level) = levels.get(key) else {
            continue;
        };
        let (hue, gamut) = match role {
            Role::Neutral => (primary_hue, NEUTRAL_GAMUT),
            Role::Primary => (primary_hue, COLOR_GAMUT),
            Role::Secondary => (secondary_hue, COLOR_GAMUT),
            Role::Tertiary => (tertiary_hue, COLOR_GAMUT),
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

/// `color` moved in lightness, away from `surface`, until the two are
/// `target` apart in contrast. Its hue and chroma are kept as far as sRGB
/// allows. On a dark surface the color gets lighter, on a light one darker.
pub fn readable_on(color: Rgb, surface: Rgb, dark: bool, target: f32) -> Rgb {
    let [mut l, c, h] = to_oklch(color);
    let mut rgb = color;
    while contrast::ratio(rgb, surface) < target && (0.0..=1.0).contains(&l) {
        l += if dark { NUDGE } else { -NUDGE };
        rgb = from_oklch(l.clamp(0.0, 1.0), c, h, COLOR_GAMUT);
    }
    rgb
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
pub(crate) fn to_oklch(rgb: Rgb) -> [f64; 3] {
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

    /// Themes as a user might write them, each hard for the standard in its
    /// own way: an accent too dark to read, a pastel light theme, no hue at
    /// all, base colors only, and a link in the accent's own color.
    const FOREIGN: &str = r##"{
      "name": "Foreign",
      "themes": [
        {
          "name": "Dark Red Accent",
          "mode": "dark",
          "colors": {
            "background": "#000000",
            "foreground": "#FFFFCE",
            "primary.background": "#720202",
            "link": "#720202",
            "base.red": "#CDA074",
            "base.blue": "#720202"
          }
        },
        {
          "name": "Pastel Light",
          "mode": "light",
          "colors": {
            "background": "#FBF1C7",
            "foreground": "#665C54",
            "primary.background": "#B8E0D2",
            "link": "#EAC4D5",
            "muted.foreground": "#C9BFA6"
          }
        },
        {
          "name": "Gray",
          "mode": "dark",
          "colors": {
            "background": "#1E1E1E",
            "foreground": "#D4D4D4",
            "primary.background": "#808080"
          }
        },
        {
          "name": "Base Colors Only",
          "mode": "light",
          "colors": {
            "background": "#FFFFFF",
            "foreground": "#000000",
            "base.red": "#FF0000",
            "base.green": "#00FF00",
            "base.yellow": "#FFFF00",
            "base.blue": "#0000FF",
            "base.magenta": "#FF00FF",
            "base.cyan": "#00FFFF"
          }
        },
        {
          "name": "Link Is Primary",
          "mode": "dark",
          "colors": {
            "background": "#002B36",
            "foreground": "#839496",
            "primary.background": "#268BD2",
            "link": "#268BD2",
            "ring": "#268BD2"
          }
        }
      ]
    }"##;

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
    fn a_color_is_moved_until_it_stands_out_and_no_further() {
        let parse = |value| contrast::parse_hex(value).unwrap();
        let (dark, light) = (parse("#16122B"), parse("#F6F4FF"));
        // A deep violet is lost on a dark surface and a pale one on a light.
        for (color, surface, is_dark) in [("#3B1D8F", dark, true), ("#D9CCFF", light, false)] {
            let before = parse(color);
            assert!(contrast::ratio(before, surface) < 3.0, "{color}");
            let after = readable_on(before, surface, is_dark, 3.0);
            assert!(contrast::ratio(after, surface) >= 3.0, "{color}");
            let (from, to) = (to_oklch(before)[2], to_oklch(after)[2]);
            let apart = (from - to).abs().min(360.0 - (from - to).abs());
            assert!(apart < 12.0, "{color}: hue moved {apart:.0} degrees");
        }
        // A color that already stands out is returned as it is.
        let fine = parse("#B299FF");
        assert_eq!(readable_on(fine, dark, true, 3.0), fine);
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

    /// Every bundled theme and every foreign one, standardized, reads as the
    /// CodeWiki themes are required to: text at AA on the page, a card and
    /// the tinted page; icons at 3:1; the page itself at the CodeWiki set's
    /// lightness.
    #[test]
    fn every_theme_meets_the_codewiki_rules_once_standardized() {
        let mut failures = Vec::new();
        for content in BUNDLED.into_iter().chain([FOREIGN]) {
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
                for key in ICON_KEYS
                    .into_iter()
                    .chain(["link", "sidebar.primary.background"])
                {
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

    /// A theme that names one accent still shows three colors, and one that
    /// names none stays gray.
    #[test]
    fn a_foreign_theme_gets_three_hues_that_stand_apart() {
        for config in configs(FOREIGN) {
            let theme = standardize(&config);
            let lch = |key| to_oklch(color(&theme, key));
            let [primary, secondary, tertiary] =
                ["primary.background", "link", "sidebar.primary.background"].map(lch);
            let name = &config.name;
            // A theme that names no accent has no hue to build on.
            if ["Gray", "Base Colors Only"].contains(&name.as_ref()) {
                for brand in [primary, secondary, tertiary] {
                    assert!(brand[1] < MIN_CHROMA, "{name} gained a hue");
                }
                continue;
            }
            let near = hue_apart(primary[2], secondary[2]);
            assert!(
                near >= SECONDARY_APART.0 - 3.0,
                "{name}: secondary {near:.0}"
            );
            let far = hue_apart(primary[2], tertiary[2]);
            assert!(far >= TERTIARY_APART - 3.0, "{name}: tertiary {far:.0}");
            let between = hue_apart(secondary[2], tertiary[2]);
            assert!(between >= BRAND_APART - 3.0, "{name}: between {between:.0}");
        }
    }

    #[test]
    fn brand_hues_come_from_the_theme_where_it_names_them() {
        let named = |pairs: &'static [(&'static str, f64)]| {
            move |key: &str| {
                pairs
                    .iter()
                    .find(|(name, _)| *name == key)
                    .map(|(_, hue)| [0.7, 0.15, *hue])
            }
        };
        // Nothing but a primary color: both others are set beside it.
        assert_eq!(
            brand_hues(named(&[("primary.background", 290.0)])),
            [Some(290.0), Some(330.0), Some(140.0)]
        );
        // A link of its own, and a sidebar accent far enough away, are kept.
        assert_eq!(
            brand_hues(named(&[
                ("primary.background", 290.0),
                ("link", 340.0),
                ("sidebar.primary.background", 120.0),
            ])),
            [Some(290.0), Some(340.0), Some(120.0)]
        );
        // A link in the primary color is no second hue; the spare colors are
        // searched: blue is the neighbour, yellow the one farthest away.
        assert_eq!(
            brand_hues(named(&[
                ("primary.background", 290.0),
                ("link", 292.0),
                ("base.blue", 255.0),
                ("base.red", 20.0),
                ("base.yellow", 95.0),
            ])),
            [Some(290.0), Some(255.0), Some(95.0)]
        );
        // The tertiary hue keeps clear of a link that sits where it would go.
        assert_eq!(
            brand_hues(named(&[("primary.background", 0.0), ("link", 150.0)])),
            [Some(0.0), Some(150.0), Some(270.0)]
        );
        assert_eq!(
            brand_hues(named(&[("primary.background", 0.0), ("link", 180.0)])),
            [Some(0.0), Some(180.0), Some(90.0)]
        );
        assert_eq!(brand_hues(named(&[])), [None; 3]);
    }

    #[test]
    fn a_dark_red_accent_keeps_its_hue_and_gains_readable_links() {
        let foreign = configs(FOREIGN)
            .into_iter()
            .find(|config| config.name.as_ref() == "Dark Red Accent")
            .unwrap();
        let theme = standardize(&foreign);
        let link = color(&theme, "link");
        let page = color(&theme, "background");
        assert!(contrast::ratio(link, page) >= AA);
        let original = to_oklch(contrast::parse_hex("#720202").unwrap())[2];
        let hue = to_oklch(color(&theme, "primary.background"))[2];
        let apart = (hue - original).abs().min(360.0 - (hue - original).abs());
        assert!(apart < 20.0, "accent hue moved {apart:.0} degrees");
    }
}
