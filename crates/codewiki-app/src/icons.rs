//! Icons, and the asset source that serves them.
//!
//! The app draws Material Icons, as Material UI apps do. Each is an SVG file
//! in `icons/`, built from the path data of `@mui/icons-material`, and is
//! embedded in the binary here.
//!
//! Every icon the app draws is a constant here. gpui-kit's own components
//! draw a few Lucide icons of their own; those come from the kit's assets.

use std::borrow::Cow;

use gpui_kit::assets::{Assets, IconNamed};
use gpui_kit::component::{ActiveTheme as _, Theme};
use gpui_kit::{App, AssetSource, Hsla, Result, SharedString};

use crate::palette::Command;

/// An icon of the app: the path its SVG is served under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph(&'static str);

impl IconNamed for Glyph {
    fn path(self) -> SharedString {
        self.0.into()
    }
}

/// Where the app's icons are served, apart from the kit's under `icons/`.
const FOLDER_PATH: &str = "icons/material/";

/// Declare the app's icons: embed each file, and name it as a constant.
macro_rules! app_icons {
    ($($name:ident = $file:literal),* $(,)?) => {
        $(pub const $name: Glyph = Glyph(concat!("icons/material/", $file, ".svg"));)*
        /// Every icon above, with its bytes.
        const EMBEDDED: &[(Glyph, &[u8])] = &[
            $(($name, include_bytes!(concat!("../icons/", $file, ".svg")))),*
        ];
    };
}

app_icons! {
    HOME = "home",
    MAP = "map",
    OVERVIEW = "dashboard",
    FOLDER = "folder",
    NOTE = "article",
    BACK = "arrow-back",
    EDIT = "edit",
    SAVE = "save",
    CLOSE = "close",
    FIGURE = "image",
    COMMANDS = "keyboard-command-key",
    GRAPH = "hub",
    HOTSPOTS = "local-fire-department",
    SYNC = "sync",
    REVIEW = "fact-check",
    THEME = "palette",
    NEW_TAB = "add",
    SEARCH = "search",
    REPOSITORY = "source",
    OWNERSHIP = "groups",
    COUPLING = "link",
    DECISIONS = "balance",
    DEAD_CODE = "delete-sweep",
    HEALTH = "monitor-heart",
    DRIFT = "difference",
    TESTS = "science",
    COVERAGE = "percent",
    SECURITY = "security",
    REFACTOR = "build",
    WORKSPACE = "workspaces",
    CONTRACTS = "handshake",
    SETTINGS = "settings",
    MOTION = "animation",
    UPDATE = "system-update-alt",
    EXTERNAL = "open-in-new",
    SUCCESS = "check-circle-outline",
    INFO = "info-outlined",
    WARNING = "report-problem-outlined",
    ERROR = "error-outline",
    EXPAND = "expand-more",
    // The side panes. Material has `ViewSidebar` only with the pane on the
    // right; the left one is the same path mirrored.
    SIDEBAR = "view-sidebar-left",
    DETAILS = "view-sidebar",
    // Pages.
    CHECK = "check",
    SYMBOL = "data-object",
    FILE = "insert-drive-file",
    // Themes. The bat is from Pictogrammers' Material Design Icons.
    THEME_VIOLET = "diamond",
    THEME_EMBER = "whatshot",
    THEME_LAGOON = "water",
    THEME_TIDE = "tsunami",
    THEME_DRACULA = "bat",
    THEME_NORD = "ac-unit",
    THEME_GRUVBOX = "album",
    THEME_ROSE_PINE = "park",
}

/// The icon that stands for the theme `name` in the theme menu. A light
/// theme shares the icon of its dark twin; a theme the app does not know,
/// such as one of the user's own, takes the palette.
pub fn for_theme(name: &str) -> Glyph {
    match name.strip_suffix(" Light").unwrap_or(name) {
        "CodeWiki Violet" => THEME_VIOLET,
        "CodeWiki Ember" => THEME_EMBER,
        "CodeWiki Lagoon" => THEME_LAGOON,
        "CodeWiki Tide" => THEME_TIDE,
        "Dracula" => THEME_DRACULA,
        "Nord" => THEME_NORD,
        "Gruvbox Dark" => THEME_GRUVBOX,
        "Rosé Pine" => THEME_ROSE_PINE,
        _ => THEME,
    }
}

/// The app's own logo, the same art as its Windows icon, served under this
/// path. Draw it with `img(icons::LOGO)`.
pub const LOGO: &str = "logo/codewiki.png";
const LOGO_BYTES: &[u8] = include_bytes!("../resources/codewiki.png");

/// The icon shown beside a palette entry.
pub fn for_command(command: &Command) -> Glyph {
    match command {
        Command::Home => HOME,
        Command::Note(_) => NOTE,
        Command::Theme(_) | Command::Themes => THEME,
        Command::Repo(_) => REPOSITORY,
        Command::Search(_) => SEARCH,
        Command::Sync => SYNC,
        Command::Graph => GRAPH,
        Command::Hotspots => HOTSPOTS,
        Command::Ownership => OWNERSHIP,
        Command::Coupling => COUPLING,
        Command::Decisions => DECISIONS,
        Command::DeadCode => DEAD_CODE,
        Command::Health => HEALTH,
        Command::Drift => DRIFT,
        Command::Tests => TESTS,
        Command::Coverage => COVERAGE,
        Command::Security => SECURITY,
        Command::Refactor => REFACTOR,
        Command::Workspace => WORKSPACE,
        Command::Contracts => CONTRACTS,
        Command::Settings => SETTINGS,
        Command::Motion(_) => MOTION,
        Command::NewTab => NEW_TAB,
        Command::CloseTab => CLOSE,
        Command::ToggleSidebar => SIDEBAR,
        Command::ToggleDetails => DETAILS,
        Command::OpenInObsidian => EXTERNAL,
        Command::Reconnect => SYNC,
        Command::OpenFolder => FOLDER,
        Command::CheckUpdates | Command::InstallUpdate => UPDATE,
    }
}

/// The theme color an icon is drawn in, so that the chrome carries the
/// theme's palette and each kind of thing keeps the same color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hue {
    /// The theme's link color, which is its secondary color.
    Link,
    /// The theme's tertiary color.
    Tertiary,
    Magenta,
    Yellow,
    Blue,
    Cyan,
    Red,
}

impl Hue {
    /// This hue in `theme`.
    pub fn color(self, theme: &Theme) -> Hsla {
        match self {
            Self::Link => theme.link,
            Self::Tertiary => theme.sidebar_primary,
            Self::Magenta => theme.magenta,
            Self::Yellow => theme.yellow,
            Self::Blue => theme.blue,
            Self::Cyan => theme.cyan,
            Self::Red => theme.red,
        }
    }
}

/// The hue of `icon` in the sidebar and the toolbar, or `None` for an icon
/// that stays neutral, such as back or close.
pub fn hue(icon: Glyph) -> Option<Hue> {
    match icon {
        HOME | MAP | NEW_TAB => Some(Hue::Link),
        OVERVIEW | THEME => Some(Hue::Magenta),
        FOLDER => Some(Hue::Yellow),
        NOTE => Some(Hue::Blue),
        GRAPH => Some(Hue::Cyan),
        HOTSPOTS => Some(Hue::Red),
        _ => None,
    }
}

/// [`hue`] in the active theme.
pub fn tint(icon: Glyph, cx: &App) -> Option<Hsla> {
    hue(icon).map(|hue| hue.color(cx.theme()))
}

/// The app's asset source: its own icons first, then gpui-kit's defaults,
/// which the kit's components need.
pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path == LOGO {
            return Ok(Some(Cow::Borrowed(LOGO_BYTES)));
        }
        match EMBEDDED.iter().find(|(glyph, _)| glyph.0 == path) {
            Some((_, bytes)) => Ok(Some(Cow::Borrowed(bytes))),
            None => Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths: Vec<SharedString> = EMBEDDED
            .iter()
            .filter(|(glyph, _)| glyph.0.starts_with(path) || FOLDER_PATH.starts_with(path))
            .map(|(glyph, _)| glyph.0.into())
            .collect();
        for path in Assets.list(path)? {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_app_icon_loads_as_svg() {
        for (glyph, _) in EMBEDDED {
            let path = glyph.path();
            assert!(path.starts_with(FOLDER_PATH), "{path}");
            let bytes = AppAssets
                .load(&path)
                .unwrap_or_else(|error| panic!("{path}: {error}"))
                .unwrap_or_else(|| panic!("{path} is not embedded"));
            assert!(bytes.starts_with(b"<svg"), "{path} is not an SVG");
            // A Material icon is drawn on a 24 unit grid and filled.
            let svg = std::str::from_utf8(&bytes).unwrap();
            assert!(svg.contains(r#"viewBox="0 0 24 24""#), "{path}");
            assert!(svg.contains("<path d=\""), "{path} has no path");
        }
    }

    #[test]
    fn the_logo_loads_as_png() {
        let bytes = AppAssets.load(LOGO).unwrap().expect("the logo is embedded");
        assert!(
            bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            "the logo is not a PNG"
        );
    }

    #[test]
    fn the_kit_default_icons_still_load() {
        // Drawn by gpui-kit's own components, not named by the app.
        for path in ["icons/chevron-right.svg", "icons/chevron-down.svg"] {
            assert!(
                matches!(AppAssets.load(path), Ok(Some(_))),
                "{path} did not load"
            );
        }
    }

    #[test]
    fn listing_has_no_duplicates() {
        let paths = AppAssets.list("icons/").unwrap();
        let mut unique = paths.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(paths.len(), unique.len());
        assert!(paths.len() > EMBEDDED.len());
    }

    #[test]
    fn each_kind_of_sidebar_row_has_its_own_hue() {
        let kinds = [MAP, OVERVIEW, FOLDER, NOTE].map(|icon| hue(icon).unwrap());
        for (index, kind) in kinds.iter().enumerate() {
            assert!(!kinds[index + 1..].contains(kind), "{kinds:?}");
        }
        assert_eq!(hue(BACK), None);
        assert_eq!(hue(CLOSE), None);
    }

    #[test]
    fn every_bundled_theme_has_an_icon_of_its_own() {
        use crate::themes::BUNDLED_NAMES;
        let dark = &BUNDLED_NAMES[..8];
        let icons: Vec<Glyph> = dark.iter().map(|name| for_theme(name)).collect();
        for (index, icon) in icons.iter().enumerate() {
            assert_ne!(*icon, THEME, "{} has no icon", dark[index]);
            assert!(!icons[index + 1..].contains(icon), "{}", dark[index]);
        }
        // A light theme shares its dark twin's icon.
        for name in &BUNDLED_NAMES[8..] {
            assert_eq!(for_theme(name), for_theme(name.trim_end_matches(" Light")));
            assert_ne!(for_theme(name), THEME, "{name}");
        }
        assert_eq!(for_theme("My Own Theme"), THEME);
    }

    #[test]
    fn an_unknown_path_is_an_error_not_a_blank() {
        assert!(AppAssets.load("icons/no-such-icon.svg").is_err());
    }
}
