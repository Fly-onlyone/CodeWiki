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
use gpui_kit::{AssetSource, Result, SharedString};

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
    EXTERNAL = "open-in-new",
    SUCCESS = "check-circle-outline",
    INFO = "info-outlined",
    WARNING = "report-problem-outlined",
    ERROR = "error-outline",
    EXPAND = "expand-more",
    // Pages.
    CHECK = "check",
    SYMBOL = "data-object",
    FILE = "insert-drive-file",
}

/// The icon shown beside a palette entry.
pub fn for_command(command: &Command) -> Glyph {
    match command {
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
        Command::OpenInObsidian => EXTERNAL,
    }
}

/// The app's asset source: its own icons first, then gpui-kit's defaults,
/// which the kit's components need.
pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
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
    fn an_unknown_path_is_an_error_not_a_blank() {
        assert!(AppAssets.load("icons/no-such-icon.svg").is_err());
    }
}
