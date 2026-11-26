//! The two tab settings: whether the app has tabs at all, and whether a
//! note the user opens goes to a tab of its own.
//!
//! Each is a file in the settings folder (`prefs`) and a global, so that a
//! click reads it without a file.

use gpui_kit::{App, Global};

use crate::prefs;

/// The setting that turns tabs on and off.
const TABS: &str = "tabs";
/// The setting that opens every note in a new tab.
const NEW_TAB: &str = "new_tab";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Settings {
    tabs: bool,
    new_tab: bool,
}

impl Global for Settings {}

impl Settings {
    /// What the two files say: tabs on and new tabs off unless the user
    /// changed them.
    fn from(tabs: Option<&str>, new_tab: Option<&str>) -> Self {
        Self {
            tabs: tabs != Some("off"),
            new_tab: new_tab == Some("on"),
        }
    }
}

/// Read the settings. Call once at start.
pub fn init(cx: &mut App) {
    cx.set_global(Settings::from(
        prefs::read(TABS).as_deref(),
        prefs::read(NEW_TAB).as_deref(),
    ));
}

fn settings(cx: &App) -> Settings {
    cx.try_global::<Settings>()
        .copied()
        .unwrap_or(Settings::from(None, None))
}

/// Whether the app shows tabs. Off, it has one view, and every note opens
/// in it.
pub fn tabs(cx: &App) -> bool {
    settings(cx).tabs
}

/// Whether the switch "Open notes in a new tab" is on, whatever `tabs` says.
pub fn new_tab_switch(cx: &App) -> bool {
    settings(cx).new_tab
}

/// Whether a note the user opens goes to a tab of its own: the switch is on
/// and so are tabs.
pub fn new_tab(cx: &App) -> bool {
    let settings = settings(cx);
    settings.tabs && settings.new_tab
}

/// Turn tabs on or off, and remember it.
pub fn set_tabs(on: bool, cx: &mut App) {
    prefs::write(TABS, word(on));
    let settings = Settings {
        tabs: on,
        ..settings(cx)
    };
    cx.set_global(settings);
}

/// Turn "Open notes in a new tab" on or off, and remember it.
pub fn set_new_tab(on: bool, cx: &mut App) {
    prefs::write(NEW_TAB, word(on));
    let settings = Settings {
        new_tab: on,
        ..settings(cx)
    };
    cx.set_global(settings);
}

fn word(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_are_on_and_new_tabs_off_until_changed() {
        assert_eq!(
            Settings::from(None, None),
            Settings {
                tabs: true,
                new_tab: false
            }
        );
    }

    #[test]
    fn each_file_turns_its_own_setting() {
        assert_eq!(
            Settings::from(Some("off"), Some("on")),
            Settings {
                tabs: false,
                new_tab: true
            }
        );
        assert_eq!(
            Settings::from(Some("on"), Some("off")),
            Settings {
                tabs: true,
                new_tab: false
            }
        );
    }

    #[test]
    fn a_word_it_does_not_know_keeps_the_default() {
        assert_eq!(
            Settings::from(Some("maybe"), Some("yes")),
            Settings::from(None, None)
        );
    }
}
