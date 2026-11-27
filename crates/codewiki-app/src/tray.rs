//! The app's icon in the notification area, with its tooltip and its menu.
//!
//! The app keeps running when its window closes; this icon is then all of it
//! that shows. A click brings the window back, a right click opens the menu.
//!
//! What the tooltip and the menu say is a [`Snapshot`] of the app, and
//! [`rows`] is the menu for one, as plain data. [`Tray::refresh`] changes
//! only the rows that differ from the last snapshot, in place: the menu may
//! be open while the app's state changes, and a menu that is rebuilt while
//! open closes under the pointer.
//!
//! Clicks arrive in a window procedure, where nothing of GPUI may be
//! touched. The two callbacks given to [`Tray::new`] must therefore only
//! pass them on (the app sends them through its event channel).

use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::status;

/// The icon in the program's resources (`resources/codewiki.rc`).
const ICON_RESOURCE: u16 = 1;
/// How many repositories "Recent repositories" lists.
pub const RECENT_ROWS: usize = 8;

/// What a row of the menu asks the app to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Show the window.
    Open,
    /// Update the wiki of the open repository.
    Sync,
    /// Show the window on the repository in this folder.
    Recent(String),
    /// Show the window on the settings.
    Settings,
    /// Look for a newer release, and show the settings, where the result is.
    CheckUpdates,
    /// Show the window on Home, which offers the newer release.
    ShowUpdate,
    /// End the app and its backend.
    Quit,
}

impl Command {
    /// The id its menu row carries.
    fn id(&self) -> String {
        match self {
            Command::Open => "open".into(),
            Command::Sync => "sync".into(),
            Command::Recent(path) => format!("recent:{path}"),
            Command::Settings => "settings".into(),
            Command::CheckUpdates => "check".into(),
            Command::ShowUpdate => "update".into(),
            Command::Quit => "quit".into(),
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "open" => Command::Open,
            "sync" => Command::Sync,
            "settings" => Command::Settings,
            "check" => Command::CheckUpdates,
            "update" => Command::ShowUpdate,
            "quit" => Command::Quit,
            _ => Command::Recent(id.strip_prefix("recent:")?.to_string()),
        })
    }
}

/// What the tray shows of the app, at one moment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// Name of the open repository.
    pub repo: Option<String>,
    /// What the app and its backend are doing (`status::state_text`).
    pub state: String,
    /// What follows "MCP:" (`status::mcp_text`).
    pub mcp: String,
    /// Whether "Update wiki now" can run: a repository is open and nothing
    /// else is running.
    pub can_sync: bool,
    /// Folders of the repositories opened last, newest first.
    pub recent: Vec<String>,
    /// The newer release that was found, if any.
    pub update: Option<String>,
}

/// One row of the menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    /// A line that only says something; it cannot be chosen.
    Label(String),
    Separator,
    Action {
        command: Command,
        text: String,
        enabled: bool,
    },
    /// "Recent repositories": one row for each folder.
    Recent(Vec<(Command, String)>),
}

/// The menu for `snapshot`, top to bottom. The rows are always the same, in
/// the same order; only their words, whether they can be chosen and the
/// folders under "Recent repositories" change. `Tray::refresh` relies on it.
pub fn rows(snapshot: &Snapshot) -> Vec<Row> {
    let action = |command: Command, text: &str, enabled: bool| Row::Action {
        command,
        text: text.to_string(),
        enabled,
    };
    vec![
        Row::Label(match &snapshot.repo {
            Some(repo) => repo.clone(),
            None => "No repository open".to_string(),
        }),
        Row::Label(snapshot.state.clone()),
        Row::Label(format!("MCP: {}", snapshot.mcp)),
        match &snapshot.update {
            Some(version) => action(
                Command::ShowUpdate,
                &format!("Update available: {version}"),
                true,
            ),
            // The same row, so that the menu keeps its shape.
            None => action(
                Command::ShowUpdate,
                &format!("{} {}", cw_core::APP_NAME, cw_core::VERSION),
                false,
            ),
        },
        Row::Separator,
        action(Command::Open, &format!("Open {}", cw_core::APP_NAME), true),
        action(Command::Sync, "Update wiki now", snapshot.can_sync),
        Row::Recent(
            snapshot
                .recent
                .iter()
                .take(RECENT_ROWS)
                .map(|path| (Command::Recent(path.clone()), recent_label(path)))
                .collect(),
        ),
        Row::Separator,
        action(Command::Settings, "Settings", true),
        action(Command::CheckUpdates, "Check for updates", true),
        Row::Separator,
        action(Command::Quit, "Quit", true),
    ]
}

/// The tooltip for `snapshot`.
pub fn tooltip(snapshot: &Snapshot) -> String {
    status::tooltip(snapshot.repo.as_deref(), &snapshot.state, &snapshot.mcp)
}

/// How a folder reads under "Recent repositories", and in the toolbar's
/// Open recent menu: its name, then where it is, since two folders may
/// share a name.
pub fn recent_label(path: &str) -> String {
    let trimmed = path.trim_end_matches(['\\', '/']);
    match trimmed.rsplit_once(['\\', '/']) {
        Some((parent, name)) if !name.is_empty() => format!("{name}  ({parent})"),
        _ => path.to_string(),
    }
}

/// `text` as a menu shows it: Windows reads `&` as "underline the next
/// letter", so one that is meant is doubled.
fn menu_text(text: &str) -> String {
    text.replace('&', "&&")
}

/// The row of the real menu that stands for a [`Row`].
enum Item {
    Plain(MenuItem),
    Recent(Submenu),
    Separator,
}

/// The icon, for as long as it lives; dropping it takes the icon away.
pub struct Tray {
    icon: TrayIcon,
    items: Vec<Item>,
    /// What the menu and the tooltip show now.
    rows: Vec<Row>,
    tooltip: String,
}

impl Tray {
    /// Put the icon in the notification area. `icon_size` is the side of a
    /// small icon on this screen, in pixels. `on_show` is called for a click
    /// on the icon and `on_command` for a row of the menu, both from a
    /// window procedure: see the module's note.
    pub fn new(
        snapshot: &Snapshot,
        icon_size: u32,
        on_show: impl Fn() + Send + Sync + 'static,
        on_command: impl Fn(Command) + Send + Sync + 'static,
    ) -> Result<Self, String> {
        let rows = rows(snapshot);
        let menu = Menu::new();
        let mut items = Vec::new();
        for row in &rows {
            let item = match row {
                Row::Label(text) => Item::Plain(MenuItem::new(menu_text(text), false, None)),
                Row::Separator => Item::Separator,
                Row::Action {
                    command,
                    text,
                    enabled,
                } => Item::Plain(MenuItem::with_id(
                    command.id(),
                    menu_text(text),
                    *enabled,
                    None,
                )),
                Row::Recent(folders) => {
                    let submenu = Submenu::new("Recent repositories", !folders.is_empty());
                    fill_recent(&submenu, folders)?;
                    Item::Recent(submenu)
                }
            };
            match &item {
                Item::Plain(item) => menu.append(item),
                Item::Recent(submenu) => menu.append(submenu),
                Item::Separator => menu.append(&PredefinedMenuItem::separator()),
            }
            .map_err(|e| format!("building the tray menu: {e}"))?;
            items.push(item);
        }
        let image = Icon::from_resource(ICON_RESOURCE, Some((icon_size, icon_size)))
            .map_err(|e| format!("loading the tray icon: {e}"))?;
        let tooltip = tooltip(snapshot);
        let icon = TrayIconBuilder::new()
            .with_icon(image)
            .with_tooltip(&tooltip)
            .with_menu(Box::new(menu))
            // A click shows the window; only the right button opens the menu.
            .with_menu_on_left_click(false)
            .build()
            .map_err(|e| format!("adding the tray icon: {e}"))?;
        // After its menu was open, the icon's window has the keyboard, and
        // Alt+F4 would destroy it and take the icon away.
        crate::win32::keep_open(icon.window_handle());
        // Only the first handler set in a process is kept, which is this one:
        // the app has one tray icon.
        TrayIconEvent::set_event_handler(Some(move |event| {
            if shows_window(&event) {
                on_show();
            }
        }));
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if let Some(command) = Command::from_id(event.id.as_ref()) {
                on_command(command);
            }
        }));
        Ok(Self {
            icon,
            items,
            rows,
            tooltip,
        })
    }

    /// Show `snapshot`: change the rows and the tooltip that differ from
    /// what is shown, and nothing else.
    pub fn refresh(&mut self, snapshot: &Snapshot) {
        let tooltip = tooltip(snapshot);
        if tooltip != self.tooltip {
            if let Err(e) = self.icon.set_tooltip(Some(&tooltip)) {
                log::warn!("tray: could not set the tooltip: {e}");
            }
            self.tooltip = tooltip;
        }
        let rows = rows(snapshot);
        for ((item, new), old) in self.items.iter().zip(&rows).zip(&self.rows) {
            if new == old {
                continue;
            }
            match (item, new) {
                (Item::Plain(item), Row::Label(text)) => item.set_text(menu_text(text)),
                (Item::Plain(item), Row::Action { text, enabled, .. }) => {
                    item.set_text(menu_text(text));
                    item.set_enabled(*enabled);
                }
                (Item::Recent(submenu), Row::Recent(folders)) => {
                    while submenu.remove_at(0).is_some() {}
                    if let Err(e) = fill_recent(submenu, folders) {
                        log::warn!("tray: {e}");
                    }
                    submenu.set_enabled(!folders.is_empty());
                }
                _ => {}
            }
        }
        self.rows = rows;
    }
}

fn fill_recent(submenu: &Submenu, folders: &[(Command, String)]) -> Result<(), String> {
    for (command, label) in folders {
        submenu
            .append(&MenuItem::with_id(
                command.id(),
                menu_text(label),
                true,
                None,
            ))
            .map_err(|e| format!("listing the recent repositories: {e}"))?;
    }
    Ok(())
}

/// Whether a click on the icon asks for the window: the left button, let go,
/// or a double click with it. The right button opens the menu by itself.
fn shows_window(event: &TrayIconEvent) -> bool {
    matches!(
        event,
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } | TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Snapshot {
        Snapshot {
            repo: Some("CodeWiki".into()),
            state: "Idle".into(),
            mcp: "active".into(),
            can_sync: true,
            recent: vec![r"D:\code\CodeWiki".into(), r"D:\code\other".into()],
            update: None,
        }
    }

    fn action(rows: &[Row], wanted: &Command) -> (String, bool) {
        rows.iter()
            .find_map(|row| match row {
                Row::Action {
                    command,
                    text,
                    enabled,
                } if command == wanted => Some((text.clone(), *enabled)),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn the_menu_starts_with_what_the_app_is_doing() {
        let rows = rows(&snapshot());
        assert_eq!(rows[0], Row::Label("CodeWiki".into()));
        assert_eq!(rows[1], Row::Label("Idle".into()));
        assert_eq!(rows[2], Row::Label("MCP: active".into()));
        // With no newer release the fourth row names the version, greyed.
        assert_eq!(
            action(&rows, &Command::ShowUpdate),
            (format!("{} {}", cw_core::APP_NAME, cw_core::VERSION), false)
        );
        assert_eq!(rows[4], Row::Separator);
        assert_eq!(
            action(&rows, &Command::Open),
            (format!("Open {}", cw_core::APP_NAME), true)
        );
        // Quit is last, so that a script reaches it with Up and Enter.
        assert!(matches!(
            rows.last(),
            Some(Row::Action {
                command: Command::Quit,
                enabled: true,
                ..
            })
        ));

        let none = rows_of(|s| s.repo = None);
        assert_eq!(none[0], Row::Label("No repository open".into()));
    }

    fn rows_of(change: impl FnOnce(&mut Snapshot)) -> Vec<Row> {
        let mut snapshot = snapshot();
        change(&mut snapshot);
        rows(&snapshot)
    }

    #[test]
    fn a_newer_release_takes_the_version_row() {
        let rows = rows_of(|s| s.update = Some("0.3.0".into()));
        assert_eq!(
            action(&rows, &Command::ShowUpdate),
            ("Update available: 0.3.0".into(), true)
        );
    }

    #[test]
    fn update_wiki_now_is_greyed_when_it_cannot_run() {
        assert!(action(&rows(&snapshot()), &Command::Sync).1);
        assert!(!action(&rows_of(|s| s.can_sync = false), &Command::Sync).1);
    }

    #[test]
    fn the_menu_keeps_its_shape_whatever_the_app_does() {
        let kinds = |rows: &[Row]| -> Vec<std::mem::Discriminant<Row>> {
            rows.iter().map(std::mem::discriminant).collect()
        };
        let usual = kinds(&rows(&snapshot()));
        for changed in [
            rows_of(|s| s.repo = None),
            rows_of(|s| s.update = Some("9.9.9".into())),
            rows_of(|s| s.recent.clear()),
            rows_of(|s| s.can_sync = false),
            rows(&Snapshot::default()),
        ] {
            assert_eq!(kinds(&changed), usual);
        }
    }

    #[test]
    fn recent_repositories_are_named_with_their_place_and_capped() {
        let rows = rows_of(|s| {
            s.recent = (0..20).map(|n| format!(r"D:\code\repo{n}")).collect();
        });
        let Some(Row::Recent(folders)) = rows.iter().find(|row| matches!(row, Row::Recent(_)))
        else {
            panic!("no recent row");
        };
        assert_eq!(folders.len(), RECENT_ROWS);
        assert_eq!(
            folders[0],
            (
                Command::Recent(r"D:\code\repo0".into()),
                r"repo0  (D:\code)".into()
            )
        );
        assert_eq!(recent_label(r"D:\code\app\"), r"app  (D:\code)");
        assert_eq!(recent_label("/home/me/app"), "app  (/home/me)");
        assert_eq!(recent_label("app"), "app");
    }

    #[test]
    fn a_row_id_names_its_command() {
        for command in [
            Command::Open,
            Command::Sync,
            Command::Recent(r"D:\code\a:b".into()),
            Command::Settings,
            Command::CheckUpdates,
            Command::ShowUpdate,
            Command::Quit,
        ] {
            assert_eq!(Command::from_id(&command.id()), Some(command));
        }
        assert_eq!(Command::from_id("something else"), None);
        // The ids muda gives rows that have none are numbers.
        assert_eq!(Command::from_id("17"), None);
    }

    #[test]
    fn an_ampersand_in_a_name_is_shown_not_read_as_a_shortcut() {
        assert_eq!(menu_text("R&D"), "R&&D");
        assert_eq!(menu_text("plain"), "plain");
    }

    #[test]
    fn only_the_left_button_asks_for_the_window() {
        let click = |button, button_state| TrayIconEvent::Click {
            id: tray_icon::TrayIconId::new("t"),
            position: Default::default(),
            rect: Default::default(),
            button,
            button_state,
        };
        assert!(shows_window(&click(
            MouseButton::Left,
            MouseButtonState::Up
        )));
        // Pressing is not yet a click, and the right button is the menu's.
        assert!(!shows_window(&click(
            MouseButton::Left,
            MouseButtonState::Down
        )));
        assert!(!shows_window(&click(
            MouseButton::Right,
            MouseButtonState::Up
        )));
        assert!(shows_window(&TrayIconEvent::DoubleClick {
            id: tray_icon::TrayIconId::new("t"),
            position: Default::default(),
            rect: Default::default(),
            button: MouseButton::Left,
        }));
    }
}
