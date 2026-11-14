//! Bundled color themes and theme switching.
//!
//! `codewiki.json` is CodeWiki's own family: four hues, each dark and light,
//! with the bundled fonts and a tighter radius. Dracula, Nord, Gruvbox and
//! Rosé Pine were written for CodeWiki from the palettes their authors
//! publish; every file carries the palette author's name and URL. User themes
//! dropped into `%APPDATA%\CodeWiki\themes` are loaded as well.

use std::rc::Rc;

use gpui_kit::component::{ActiveTheme as _, Theme, ThemeRegistry};
use gpui_kit::{App, Global, Pixels, SharedString};

use crate::{prefs, standard};

/// Name of the setting that remembers the chosen theme.
const SETTING: &str = "theme";

/// Theme applied on first start.
const DEFAULT_THEME: &str = "CodeWiki Violet";

/// Theme applied when the saved one is gone and its name says it was light.
const DEFAULT_LIGHT_THEME: &str = "CodeWiki Violet Light";

/// The bundled themes, in the order the theme menu lists them. The menu
/// groups them by mode, so the dark ones come first.
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: SharedString,
    pub dark: bool,
}

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
    // Every theme is drawn on the CodeWiki set's levels, in its own hues.
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
            .map(|theme| (theme.name.clone(), theme.mode.is_dark())),
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

/// The themes on offer, from every loaded theme as `(name, is dark)`: the
/// bundled ones in `BUNDLED_NAMES` order, then the user's own by name.
fn catalog(loaded: impl Iterator<Item = (SharedString, bool)>) -> Vec<Entry> {
    let position = |name: &str| BUNDLED_NAMES.iter().position(|bundled| *bundled == name);
    let mut entries: Vec<Entry> = loaded.map(|(name, dark)| Entry { name, dark }).collect();
    entries.sort_by(|a, b| {
        let (a_at, b_at) = (position(&a.name), position(&b.name));
        a_at.is_none()
            .cmp(&b_at.is_none())
            .then(a_at.cmp(&b_at))
            .then_with(|| a.name.cmp(&b.name))
    });
    entries
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
        resolve,
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

    fn offered() -> Vec<Entry> {
        BUNDLED_NAMES
            .iter()
            .enumerate()
            .map(|(index, name)| Entry {
                name: (*name).into(),
                dark: index < 8,
            })
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
    fn the_catalog_lists_bundled_themes_in_menu_order_then_the_users() {
        let loaded = [
            ("Zebra", true),
            ("Nord", true),
            ("CodeWiki Tide Light", false),
            ("Apricot", false),
            ("CodeWiki Violet", true),
        ];
        let entries = catalog(loaded.into_iter().map(|(name, dark)| (name.into(), dark)));
        let names: Vec<&str> = entries.iter().map(|entry| entry.name.as_ref()).collect();
        assert_eq!(
            names,
            [
                "CodeWiki Violet",
                "Nord",
                "CodeWiki Tide Light",
                "Apricot",
                "Zebra"
            ]
        );
        assert!(!entries[3].dark);
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
    fn the_codewiki_family_names_the_bundled_fonts() {
        for theme in codewiki_themes() {
            assert_eq!(theme["font.family"], fonts::UI_FAMILY);
            assert_eq!(theme["mono_font.family"], fonts::MONO_FAMILY);
        }
    }

    /// Every color that carries text in the reading pane must pass WCAG AA
    /// there, whatever is behind the window.
    #[test]
    fn the_codewiki_family_passes_aa_on_the_reading_pane() {
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
        let mut failures = Vec::new();
        for theme in codewiki_themes() {
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
            let button = ratio(color("primary.foreground"), color("primary.background"));
            if button < AA {
                failures.push(format!("{name}: primary button text is {button:.2}:1"));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    /// Icons in the sidebar and the toolbar are drawn in these colors
    /// (`icons::Hue`). An icon is not text, so it needs WCAG's 3:1 for
    /// graphics (1.4.11), on the page and in the sidebar.
    #[test]
    fn the_codewiki_icon_hues_stand_out_from_their_surfaces() {
        const NON_TEXT: f32 = 3.0;
        const ICON_KEYS: [&str; 7] = [
            "link",
            "base.red",
            "base.green",
            "base.yellow",
            "base.blue",
            "base.magenta",
            "base.cyan",
        ];
        let mut failures = Vec::new();
        for theme in codewiki_themes() {
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
