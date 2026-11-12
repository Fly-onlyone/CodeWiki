//! Bundled color themes and theme switching.
//!
//! `codewiki.json` is CodeWiki's own family: four hues, each dark and light,
//! with the bundled fonts and a tighter radius. Most other theme files come
//! from the gpui-kit project (Apache-2.0). Dracula, GitHub, Nord, One and Rosé
//! Pine were written for CodeWiki from the palettes their authors publish.
//! Every file carries the palette author's name and URL. User themes dropped
//! into `%APPDATA%\CodeWiki\themes` are loaded as well.

use gpui_kit::component::{ActiveTheme as _, Theme, ThemeRegistry};
use gpui_kit::{App, Global, Pixels, SharedString};

/// Theme applied on first start.
const DEFAULT_THEME: &str = "CodeWiki Violet";

/// The CodeWiki family. Kept apart from `BUNDLED` so tests can check it alone.
const CODEWIKI: &str = include_str!("../themes/codewiki.json");

const BUNDLED: [&str; 27] = [
    CODEWIKI,
    include_str!("../themes/dracula.json"),
    include_str!("../themes/github.json"),
    include_str!("../themes/nord.json"),
    include_str!("../themes/one.json"),
    include_str!("../themes/rose-pine.json"),
    include_str!("../themes/adventure.json"),
    include_str!("../themes/alduin.json"),
    include_str!("../themes/asciinema.json"),
    include_str!("../themes/aurora.json"),
    include_str!("../themes/ayu.json"),
    include_str!("../themes/catppuccin.json"),
    include_str!("../themes/everforest.json"),
    include_str!("../themes/fahrenheit.json"),
    include_str!("../themes/flexoki.json"),
    include_str!("../themes/gruvbox.json"),
    include_str!("../themes/harper.json"),
    include_str!("../themes/hybrid.json"),
    include_str!("../themes/jellybeans.json"),
    include_str!("../themes/kibble.json"),
    include_str!("../themes/macos-classic.json"),
    include_str!("../themes/mellifluous.json"),
    include_str!("../themes/molokai.json"),
    include_str!("../themes/solarized.json"),
    include_str!("../themes/spaceduck.json"),
    include_str!("../themes/tokyonight.json"),
    include_str!("../themes/twilight.json"),
];

fn user_theme_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("APPDATA").map(|dir| {
        std::path::PathBuf::from(dir)
            .join("CodeWiki")
            .join("themes")
    })
}

fn state_file() -> Option<std::path::PathBuf> {
    std::env::var_os("APPDATA").map(|dir| {
        std::path::PathBuf::from(dir)
            .join("CodeWiki")
            .join("theme.txt")
    })
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
    let saved = state_file().and_then(|f| std::fs::read_to_string(f).ok());
    let wanted = saved.as_deref().map(str::trim).unwrap_or(DEFAULT_THEME);
    if !apply(wanted, cx) {
        apply(DEFAULT_THEME, cx);
    }
}

/// Every loaded theme as `(name, is dark)`, in name order.
pub fn list(cx: &App) -> Vec<(String, bool)> {
    ThemeRegistry::global(cx)
        .sorted_themes()
        .iter()
        .map(|t| (t.name.to_string(), t.mode.is_dark()))
        .collect()
}

/// Apply a theme by name and remember the choice. Returns false when no
/// theme has that name.
pub fn select(name: &str, cx: &mut App) -> bool {
    if !apply(name, cx) {
        return false;
    }
    if let Some(file) = state_file() {
        if let Some(parent) = file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(file, name);
    }
    true
}

/// Switch to the next theme in name order and remember the choice.
#[allow(dead_code)]
pub fn next_theme(cx: &mut App) {
    let current = cx.theme().theme_name().to_string();
    let names: Vec<String> = ThemeRegistry::global(cx)
        .sorted_themes()
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    if names.is_empty() {
        return;
    }
    let position = names
        .iter()
        .position(|n| *n == current)
        .map(|i| i + 1)
        .unwrap_or(0);
    let next = &names[position % names.len()];
    if apply(next, cx)
        && let Some(file) = state_file()
    {
        if let Some(parent) = file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(file, next);
    }
}

#[cfg(test)]
mod tests {
    use super::{BUNDLED, CODEWIKI, DEFAULT_THEME};
    use crate::contrast::{AA, PANE_FLOOR, parse_hex, ratio, worst_ratio};
    use crate::fonts;
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
            let pane = color("background");
            for key in TEXT_KEYS {
                let worst = worst_ratio(color(key), pane, PANE_FLOOR);
                if worst < AA {
                    failures.push(format!("{name}: {key} is {worst:.2}:1"));
                }
            }
            let button = ratio(color("primary.foreground"), color("primary.background"));
            if button < AA {
                failures.push(format!("{name}: primary button text is {button:.2}:1"));
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
