//! Bundled color themes and theme switching.
//!
//! `codewiki.json` is CodeWiki's own family: four hues, each dark and light,
//! with the bundled fonts and a tighter radius. Dracula, Nord, Gruvbox and
//! Rosé Pine were written for CodeWiki from the palettes their authors
//! publish; every file carries the palette author's name and URL. Their
//! backgrounds, surfaces, borders and fills are the published values. A
//! color drawn as text or as an icon keeps its published hue and is as much
//! lighter as it takes to be read on the app's glass.
//!
//! User themes dropped into `%APPDATA%\CodeWiki\themes` are loaded as well,
//! and redrawn on the CodeWiki set's levels (`standard.rs`).

use std::rc::Rc;

use gpui_kit::component::{ActiveTheme as _, Theme, ThemeConfig, ThemeRegistry};
use gpui_kit::{App, Global, Hsla, Pixels, Rgba, SharedString};

use crate::contrast::{self, Rgb};
use crate::{prefs, standard};

/// Name of the setting that remembers the chosen theme.
const SETTING: &str = "theme";

/// Theme applied on first start.
const DEFAULT_THEME: &str = "CodeWiki Violet";

/// Theme applied when the saved one is gone and its name says it was light.
const DEFAULT_LIGHT_THEME: &str = "CodeWiki Violet Light";

/// The bundled themes, in the order the theme menu lists them: the dark
/// ones, then the light ones.
pub(crate) const BUNDLED_NAMES: [&str; 12] = [
    "CodeWiki Violet",
    "CodeWiki Ember",
    "CodeWiki Lagoon",
    "CodeWiki Tide",
    "Dracula",
    "Nord",
    "Gruvbox Dark",
    "Rosé Pine",
    "CodeWiki Violet Light",
    "CodeWiki Ember Light",
    "CodeWiki Lagoon Light",
    "CodeWiki Tide Light",
];

/// The CodeWiki family. Kept apart from `BUNDLED` so tests can check it alone.
pub(crate) const CODEWIKI: &str = include_str!("../themes/codewiki.json");

pub(crate) const BUNDLED: [&str; 5] = [
    CODEWIKI,
    include_str!("../themes/dracula.json"),
    include_str!("../themes/nord.json"),
    include_str!("../themes/gruvbox.json"),
    include_str!("../themes/rose-pine.json"),
];

/// A theme the user can choose.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub name: SharedString,
    pub dark: bool,
    /// Samples of the theme as it is drawn: its page, then its primary,
    /// secondary and tertiary colors.
    pub swatches: [Hsla; 4],
}

/// The three colors a theme is known by. Each has its own work, so that a
/// theme shows more than one hue:
///
/// - `primary` fills what the user acts on and what is chosen: buttons, the
///   active tab, the selection, the focus ring.
/// - `secondary` is the color of links and of what is drawn as one: section
///   labels, a note's title, list markers.
/// - `tertiary` marks what stands apart from both: the bar of the selected
///   row, kind labels, quotes, table heads and second-level headings.
///
/// A theme file carries them in keys the kit already reads:
/// `primary.background`, `link` and `sidebar.primary.background`. The kit
/// draws nothing of its own in the third.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Brand {
    pub primary: Hsla,
    pub secondary: Hsla,
    pub tertiary: Hsla,
}

impl Brand {
    pub fn of(theme: &Theme) -> Self {
        Self {
            primary: theme.primary,
            secondary: theme.link,
            tertiary: theme.sidebar_primary,
        }
    }
}

/// The brand colors of the active theme.
pub fn brand(cx: &App) -> Brand {
    Brand::of(cx.theme())
}

/// An icon needs WCAG's 3:1 for graphics against what it is drawn on.
const ICON_CONTRAST: f32 = 3.0;

/// The themes on offer, built once at start.
struct Catalog(Vec<Entry>);

impl Global for Catalog {}

fn user_theme_dir() -> Option<std::path::PathBuf> {
    prefs::dir().map(|dir| dir.join("themes"))
}

/// Fonts, radius and shadow as they were before any theme was applied.
///
/// A theme file sets these only when it names them, and the values then stay
/// for every later theme. Without a reset, a theme that names none of them
/// would inherit the fonts and radius of whichever theme was active before.
#[derive(Clone)]
struct Baseline {
    font_family: SharedString,
    font_size: Pixels,
    mono_font_family: SharedString,
    mono_font_size: Pixels,
    radius: Pixels,
    radius_lg: Pixels,
    shadow: bool,
}

impl Global for Baseline {}

impl Baseline {
    fn capture(theme: &Theme) -> Self {
        Self {
            font_family: theme.font_family.clone(),
            font_size: theme.font_size,
            mono_font_family: theme.mono_font_family.clone(),
            mono_font_size: theme.mono_font_size,
            radius: theme.radius,
            radius_lg: theme.radius_lg,
            shadow: theme.shadow,
        }
    }

    fn restore(self, theme: &mut Theme) {
        theme.font_family = self.font_family;
        theme.font_size = self.font_size;
        theme.mono_font_family = self.mono_font_family;
        theme.mono_font_size = self.mono_font_size;
        theme.radius = self.radius;
        theme.radius_lg = self.radius_lg;
        theme.shadow = self.shadow;
    }
}

fn apply(name: &str, cx: &mut App) -> bool {
    let Some(config) = ThemeRegistry::global(cx).themes().get(name).cloned() else {
        return false;
    };
    // A bundled theme is drawn as its file has it; one of the user's own, on
    // the CodeWiki set's levels in its own hues.
    let config = Rc::new(standard::standardize(&config));
    let baseline = cx.try_global::<Baseline>().cloned();
    Theme::update(cx, |theme| {
        if let Some(baseline) = baseline {
            baseline.restore(theme);
        }
        theme.apply_config(&config);
    });
    cx.refresh_windows();
    true
}

/// Load bundled and user themes, then apply the saved or default theme.
pub fn init(cx: &mut App) {
    let baseline = Baseline::capture(cx.theme());
    cx.set_global(baseline);
    for content in BUNDLED {
        if let Err(error) = ThemeRegistry::global_mut(cx).load_themes_from_str(content) {
            eprintln!("codewiki: skipping a bundled theme: {error}");
        }
    }
    if let Some(dir) = user_theme_dir()
        && let Ok(entries) = std::fs::read_dir(&dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json")
                && let Ok(content) = std::fs::read_to_string(&path)
                && let Err(error) = ThemeRegistry::global_mut(cx).load_themes_from_str(&content)
            {
                eprintln!("codewiki: skipping theme {}: {error}", path.display());
            }
        }
    }
    // The kit's own two themes stay loaded but are not on offer.
    let entries = catalog(
        ThemeRegistry::global(cx)
            .sorted_themes()
            .iter()
            .filter(|theme| !theme.is_default)
            .map(|theme| Entry {
                name: theme.name.clone(),
                dark: theme.mode.is_dark(),
                swatches: swatches(&standard::standardize(theme)),
            }),
    );
    let (wanted, stale) = resolve(prefs::read(SETTING).as_deref(), &entries);
    cx.set_global(Catalog(entries));
    if !apply(&wanted, cx) {
        apply(DEFAULT_THEME, cx);
    }
    if stale {
        prefs::write(SETTING, &wanted);
    }
}

/// The themes on offer, in menu order: the dark ones, then the light ones.
/// In each mode the bundled themes come first, in `BUNDLED_NAMES` order,
/// then the user's own by name.
fn catalog(loaded: impl Iterator<Item = Entry>) -> Vec<Entry> {
    let position = |name: &str| BUNDLED_NAMES.iter().position(|bundled| *bundled == name);
    let mut entries: Vec<Entry> = loaded.collect();
    entries.sort_by(|a, b| {
        let (a_at, b_at) = (position(&a.name), position(&b.name));
        b.dark
            .cmp(&a.dark)
            .then(a_at.is_none().cmp(&b_at.is_none()))
            .then(a_at.cmp(&b_at))
            .then_with(|| a.name.cmp(&b.name))
    });
    entries
}

fn to_hsla(rgb: Rgb) -> Hsla {
    Rgba {
        r: rgb[0],
        g: rgb[1],
        b: rgb[2],
        a: 1.0,
    }
    .into()
}

fn to_rgb(color: Hsla) -> Rgb {
    let rgba = Rgba::from(color);
    [rgba.r, rgba.g, rgba.b]
}

/// The samples of a theme for the menu. A color the theme does not set is
/// drawn as the kit would draw it: a link or a sidebar accent falls back to
/// the primary color.
fn swatches(config: &ThemeConfig) -> [Hsla; 4] {
    let colors = &config.colors;
    let read = |value: &Option<SharedString>| value.as_deref().and_then(contrast::parse_hex);
    let gray = [0.5; 3];
    let primary = read(&colors.primary).unwrap_or(gray);
    [
        read(&colors.background).unwrap_or(gray),
        primary,
        read(&colors.link).unwrap_or(primary),
        read(&colors.sidebar_primary).unwrap_or(primary),
    ]
    .map(to_hsla)
}

/// `color` for an icon on the active theme's menu. The icon of a theme in
/// the menu is drawn in that theme's primary color, which was chosen for
/// its own surfaces, not for these; it is lightened or darkened here, in its
/// own hue, until it stands out.
pub fn glyph_tint(color: Hsla, cx: &App) -> Hsla {
    let theme = cx.theme();
    to_hsla(standard::readable_on(
        to_rgb(color),
        to_rgb(theme.popover),
        theme.is_dark(),
        ICON_CONTRAST,
    ))
}

/// The theme to start with, and whether the saved name must be replaced. A
/// saved theme that is no longer on offer gives way to the default of its
/// mode, as far as its name tells the mode.
fn resolve(saved: Option<&str>, entries: &[Entry]) -> (String, bool) {
    match saved {
        None => (DEFAULT_THEME.to_string(), false),
        Some(name) if entries.iter().any(|entry| entry.name.as_ref() == name) => {
            (name.to_string(), false)
        }
        Some(name) if name.ends_with(" Light") => (DEFAULT_LIGHT_THEME.to_string(), true),
        Some(_) => (DEFAULT_THEME.to_string(), true),
    }
}

/// The themes the user can choose, in menu order.
pub fn entries(cx: &App) -> &[Entry] {
    cx.try_global::<Catalog>()
        .map_or(&[], |catalog| catalog.0.as_slice())
}

/// Apply a theme by name and remember the choice. Returns false when no
/// theme has that name.
pub fn select(name: &str, cx: &mut App) -> bool {
    if !apply(name, cx) {
        return false;
    }
    prefs::write(SETTING, name);
    true
}

#[cfg(test)]
mod tests {
    use super::{
        BUNDLED, BUNDLED_NAMES, CODEWIKI, DEFAULT_LIGHT_THEME, DEFAULT_THEME, Entry, catalog,
        resolve, swatches, to_hsla,
    };
    use crate::contrast::{AA, BLACK, PANE_FLOOR, WHITE, over, parse_hex, ratio, worst_ratio};
    use crate::fonts;
    use crate::glass::PANE_TINT;
    use gpui_kit::component::ThemeConfigColors;
    use std::collections::BTreeSet;

    fn codewiki_themes() -> Vec<serde_json::Value> {
        let set: serde_json::Value = serde_json::from_str(CODEWIKI).unwrap();
        set["themes"].as_array().unwrap().clone()
    }

    fn bundled_themes() -> Vec<serde_json::Value> {
        BUNDLED
            .into_iter()
            .flat_map(|content| {
                let set: serde_json::Value = serde_json::from_str(content).unwrap();
                set["themes"].as_array().unwrap().clone()
            })
            .collect()
    }

    #[test]
    fn the_codewiki_family_is_four_hues_each_dark_and_light() {
        let themes = codewiki_themes();
        let names: Vec<&str> = themes.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(names.len(), 8);
        assert!(names.contains(&DEFAULT_THEME));
        for theme in &themes {
            let name = theme["name"].as_str().unwrap();
            let light = name.ends_with(" Light");
            assert_eq!(
                theme["mode"],
                if light { "light" } else { "dark" },
                "{name}"
            );
            let twin = if light {
                name.trim_end_matches(" Light").to_string()
            } else {
                format!("{name} Light")
            };
            assert!(names.contains(&twin.as_str()), "{name} has no twin");
        }
    }

    fn entry(name: &str, dark: bool) -> Entry {
        Entry {
            name: name.to_string().into(),
            dark,
            swatches: [gpui_kit::black(); 4],
        }
    }

    fn offered() -> Vec<Entry> {
        BUNDLED_NAMES
            .iter()
            .enumerate()
            .map(|(index, name)| entry(name, index < 8))
            .collect()
    }

    #[test]
    fn the_bundled_themes_are_the_twelve_on_the_menu() {
        let mut found = Vec::new();
        for content in BUNDLED {
            let set: serde_json::Value = serde_json::from_str(content).unwrap();
            for theme in set["themes"].as_array().unwrap() {
                let name = theme["name"].as_str().unwrap().to_string();
                found.push((name, theme["mode"] == "dark"));
            }
        }
        found.sort();
        // Eight dark themes, then the four light ones.
        let mut expected: Vec<(String, bool)> = offered()
            .into_iter()
            .map(|entry| (entry.name.to_string(), entry.dark))
            .collect();
        expected.sort();
        assert_eq!(found, expected);
        assert!(BUNDLED_NAMES.contains(&DEFAULT_THEME));
        assert!(BUNDLED_NAMES.contains(&DEFAULT_LIGHT_THEME));
    }

    #[test]
    fn the_catalog_lists_each_mode_with_bundled_themes_before_the_users() {
        let loaded = [
            ("Zebra", true),
            ("Nord", true),
            ("CodeWiki Tide Light", false),
            ("Apricot", false),
            ("CodeWiki Violet", true),
        ];
        let entries = catalog(loaded.into_iter().map(|(name, dark)| entry(name, dark)));
        let listed: Vec<(&str, bool)> = entries
            .iter()
            .map(|entry| (entry.name.as_ref(), entry.dark))
            .collect();
        assert_eq!(
            listed,
            [
                ("CodeWiki Violet", true),
                ("Nord", true),
                ("Zebra", true),
                ("CodeWiki Tide Light", false),
                ("Apricot", false),
            ]
        );
    }

    #[test]
    fn a_theme_is_sampled_by_its_page_and_its_three_hues() {
        let theme = |colors: serde_json::Value| -> gpui_kit::component::ThemeConfig {
            serde_json::from_value(serde_json::json!({
                "name": "Sample",
                "mode": "dark",
                "colors": colors,
            }))
            .unwrap()
        };
        let hex = |value: &str| to_hsla(crate::contrast::parse_hex(value).unwrap());
        let full = swatches(&theme(serde_json::json!({
            "background": "#101010",
            "primary.background": "#7C4DEE",
            "link": "#F59F2E",
            "sidebar.primary.background": "#2DD4A7",
        })));
        assert_eq!(
            full,
            [
                hex("#101010"),
                hex("#7C4DEE"),
                hex("#F59F2E"),
                hex("#2DD4A7")
            ]
        );
        // The kit draws a missing link or sidebar accent in the primary color.
        let bare = swatches(&theme(serde_json::json!({
            "background": "#101010",
            "primary.background": "#7C4DEE",
        })));
        assert_eq!(bare[2], hex("#7C4DEE"));
        assert_eq!(bare[3], hex("#7C4DEE"));
    }

    #[test]
    fn a_saved_theme_that_is_gone_gives_way_to_the_default_of_its_mode() {
        let entries = offered();
        assert_eq!(resolve(None, &entries), (DEFAULT_THEME.to_string(), false));
        assert_eq!(resolve(Some("Nord"), &entries), ("Nord".to_string(), false));
        assert_eq!(
            resolve(Some("One Dark"), &entries),
            (DEFAULT_THEME.to_string(), true)
        );
        assert_eq!(
            resolve(Some("One Light"), &entries),
            (DEFAULT_LIGHT_THEME.to_string(), true)
        );
        // The kit's own themes are loaded but not on offer.
        assert_eq!(
            resolve(Some("Default Dark"), &entries),
            (DEFAULT_THEME.to_string(), true)
        );
    }

    #[test]
    fn every_bundled_theme_names_the_bundled_fonts() {
        for theme in bundled_themes() {
            assert_eq!(theme["font.family"], fonts::UI_FAMILY);
            assert_eq!(theme["mono_font.family"], fonts::MONO_FAMILY);
        }
    }

    /// Every color that carries text in the reading pane must pass WCAG AA
    /// there, whatever is behind the window.
    #[test]
    fn every_bundled_theme_passes_aa_on_the_reading_pane() {
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
        let mut failures = Vec::new();
        for theme in bundled_themes() {
            let name = theme["name"].as_str().unwrap();
            let color = |key: &str| {
                let hex = theme["colors"][key]
                    .as_str()
                    .unwrap_or_else(|| panic!("{name} has no {key}"));
                parse_hex(hex).unwrap_or_else(|| panic!("{name}: {key} is not a hex color"))
            };
            // The page, a card on it (drawn in the secondary color, as dense
            // as the page), and the page with its primary tint.
            let tinted_page = over(color("primary.background"), PANE_TINT, color("background"));
            let surfaces = [
                ("background", color("background")),
                ("secondary.background", color("secondary.background")),
                ("the tinted page", tinted_page),
            ];
            for (surface, fill) in surfaces {
                for key in TEXT_KEYS {
                    let worst = worst_ratio(color(key), fill, PANE_FLOOR, [BLACK, WHITE]);
                    if worst < AA {
                        failures.push(format!("{name}: {key} on {surface} is {worst:.2}:1"));
                    }
                }
            }
            for fill in ["primary.background", "sidebar.primary.background"] {
                let text = fill.replace("background", "foreground");
                let on_fill = ratio(color(&text), color(fill));
                if on_fill < AA {
                    failures.push(format!("{name}: text on {fill} is {on_fill:.2}:1"));
                }
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    /// Icons in the sidebar and the toolbar are drawn in these colors
    /// (`icons::Hue`). An icon is not text, so it needs WCAG's 3:1 for
    /// graphics (1.4.11), on the page and in the sidebar.
    #[test]
    fn every_bundled_theme_has_icon_hues_that_stand_out() {
        const NON_TEXT: f32 = 3.0;
        const ICON_KEYS: [&str; 8] = [
            "link",
            "sidebar.primary.background",
            "base.red",
            "base.green",
            "base.yellow",
            "base.blue",
            "base.magenta",
            "base.cyan",
        ];
        let mut failures = Vec::new();
        for theme in bundled_themes() {
            let name = theme["name"].as_str().unwrap();
            let color = |key: &str| parse_hex(theme["colors"][key].as_str().unwrap()).unwrap();
            for surface in ["background", "sidebar.background"] {
                for key in ICON_KEYS {
                    let worst = worst_ratio(color(key), color(surface), PANE_FLOOR, [BLACK, WHITE]);
                    if worst < NON_TEXT {
                        failures.push(format!("{name}: {key} on {surface} is {worst:.2}:1"));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    /// A theme is told by three hues, not one: the secondary color is a
    /// neighbour of the primary one, the tertiary color sits far from both,
    /// and neither can be taken for a status color.
    #[test]
    fn every_bundled_theme_has_three_brand_hues_that_stand_apart() {
        use crate::standard::{
            BRAND_APART, MIN_CHROMA, SECONDARY_APART, TERTIARY_APART, hue_apart, to_oklch,
        };
        /// The least a brand hue sits from a status hue of the same theme.
        const STATUS_APART: f64 = 30.0;
        /// Rosé Pine publishes six hues, and a theme has seven jobs for
        /// them: three brand colors and four status colors. Its rose sits
        /// beside its red, and its foam serves as both tertiary and info.
        const TOO_FEW_HUES: [&str; 1] = ["Rosé Pine"];
        let mut failures = Vec::new();
        for theme in bundled_themes() {
            let name = theme["name"].as_str().unwrap();
            let hue = |key: &str| {
                let [_, chroma, hue] =
                    to_oklch(parse_hex(theme["colors"][key].as_str().unwrap()).unwrap());
                assert!(chroma >= MIN_CHROMA, "{name}: {key} is gray");
                hue
            };
            let primary = hue("primary.background");
            let secondary = hue("link");
            let tertiary = hue("sidebar.primary.background");
            let near = hue_apart(primary, secondary);
            if !(SECONDARY_APART.0..=SECONDARY_APART.1).contains(&near) {
                failures.push(format!("{name}: secondary is {near:.0} from primary"));
            }
            let far = hue_apart(primary, tertiary);
            if far < TERTIARY_APART {
                failures.push(format!("{name}: tertiary is {far:.0} from primary"));
            }
            let between = hue_apart(secondary, tertiary);
            if between < BRAND_APART {
                failures.push(format!("{name}: tertiary is {between:.0} from secondary"));
            }
            let statuses: &[&str] = if TOO_FEW_HUES.contains(&name) {
                &[]
            } else {
                &["success", "warning", "danger", "info"]
            };
            for status in statuses {
                let status_hue = hue(&format!("{status}.background"));
                for (brand, brand_hue) in [("secondary", secondary), ("tertiary", tertiary)] {
                    let apart = hue_apart(brand_hue, status_hue);
                    if apart < STATUS_APART {
                        failures.push(format!("{name}: {brand} is {apart:.0} from {status}"));
                    }
                }
            }
            // The link and its two states are one color.
            for state in ["link.hover", "link.active"] {
                assert_eq!(theme["colors"][state], theme["colors"]["link"], "{name}");
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    /// In the theme menu, the icon of each theme is drawn on the menu of
    /// whichever theme is in use.
    #[test]
    fn a_theme_icon_stands_out_on_the_menu_of_every_other_theme() {
        use crate::standard::readable_on;
        let themes = bundled_themes();
        let color = |theme: &serde_json::Value, key: &str| {
            parse_hex(theme["colors"][key].as_str().unwrap()).unwrap()
        };
        for active in &themes {
            let menu = color(active, "popover.background");
            let dark = active["mode"] == "dark";
            for listed in &themes {
                let icon = readable_on(
                    color(listed, "primary.background"),
                    menu,
                    dark,
                    super::ICON_CONTRAST,
                );
                assert!(
                    ratio(icon, menu) >= super::ICON_CONTRAST,
                    "{} on the menu of {}",
                    listed["name"],
                    active["name"]
                );
            }
        }
    }

    /// The theme loader ignores color keys it does not know, so a misspelt key
    /// is a color that silently never applies.
    #[test]
    fn bundled_themes_use_only_schema_color_keys() {
        let schema = serde_json::to_value(ThemeConfigColors::default()).unwrap();
        let known: BTreeSet<&str> = schema
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert!(known.contains("link.hover"), "schema keys were not read");

        let mut unknown = BTreeSet::new();
        for content in BUNDLED {
            let set: serde_json::Value = serde_json::from_str(content).unwrap();
            for theme in set["themes"].as_array().unwrap() {
                let name = theme["name"].as_str().unwrap();
                for key in theme["colors"].as_object().unwrap().keys() {
                    if !known.contains(key.as_str()) {
                        unknown.insert(format!("{name}: {key}"));
                    }
                }
            }
        }
        assert!(unknown.is_empty(), "unknown color keys: {unknown:#?}");
    }
}
