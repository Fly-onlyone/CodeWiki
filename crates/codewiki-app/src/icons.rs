//! Icons, and the asset source that serves them.
//!
//! gpui-kit ships the whole Lucide set but embeds only the hundred or so its
//! own components draw. This module embeds the ones the app adds, by name, so
//! the binary does not carry the other 1,700.
//!
//! Every icon the app draws is a constant here. Naming one anywhere else
//! would bypass the list below and draw nothing.

use std::borrow::Cow;

use gpui_kit::assets::{Assets, IconName, icon_assets};
use gpui_kit::{AssetSource, Result, SharedString};

use crate::palette::Command;

/// Declare the app's icons: embed each one, and name it as a constant.
macro_rules! app_icons {
    ($($name:ident = $icon:ident),* $(,)?) => {
        icon_assets!(Embedded, [$($icon),*]);
        $(pub const $name: IconName = IconName::$icon;)*
        /// Every icon above, for the test that loads them all.
        #[cfg(test)]
        const ALL: &[IconName] = &[$($name),*];
    };
}

app_icons! {
    // Sidebar.
    HOME = House,
    OVERVIEW = LayoutDashboard,
    FOLDER = Folder,
    NOTE = FileText,
    // Toolbar and tabs.
    BACK = ArrowLeft,
    EDIT = Pencil,
    SAVE = Save,
    CLOSE = X,
    FIGURE = Image,
    COMMANDS = Command,
    GRAPH = Network,
    HOTSPOTS = Flame,
    SYNC = RefreshCw,
    REVIEW = ClipboardCheck,
    THEME = Palette,
    NEW_TAB = Plus,
    // Palette commands that have no toolbar button.
    SEARCH = Search,
    REPOSITORY = GitBranch,
    OWNERSHIP = Users,
    COUPLING = Link2,
    DECISIONS = Scale,
    DEAD_CODE = Ghost,
    HEALTH = HeartPulse,
    DRIFT = FileDiff,
    TESTS = FlaskConical,
    COVERAGE = Percent,
    SECURITY = ShieldCheck,
    REFACTOR = Wrench,
    WORKSPACE = Boxes,
    CONTRACTS = Handshake,
    SETTINGS = Settings,
    MOTION = Zap,
    EXTERNAL = ExternalLink,
}

/// The arrow of a folder row. gpui-kit embeds this one for its own menus.
pub const EXPAND: IconName = IconName::ChevronDown;

/// The icon shown beside a palette entry.
pub fn for_command(command: &Command) -> IconName {
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
        match Embedded.load(path)? {
            Some(bytes) => Ok(Some(bytes)),
            None => Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = Embedded.list(path)?;
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
        for icon in ALL {
            let path = icon.path();
            let bytes = AppAssets
                .load(&path)
                .unwrap_or_else(|error| panic!("{path}: {error}"))
                .unwrap_or_else(|| panic!("{path} is not embedded"));
            assert!(bytes.starts_with(b"<svg"), "{path} is not an SVG");
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
        assert!(paths.len() > ALL.len());
    }

    #[test]
    fn an_unknown_path_is_an_error_not_a_blank() {
        assert!(AppAssets.load("icons/no-such-icon.svg").is_err());
    }
}
