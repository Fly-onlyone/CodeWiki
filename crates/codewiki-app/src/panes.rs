//! The two side panes: whether each is shown.
//!
//! The sidebar and the details pane can each be hidden to leave the page
//! more room. The choice is remembered between runs, one setting per pane.

use crate::prefs;

/// What the setting of a hidden pane holds. Anything else, a missing setting
/// included, means the pane is shown.
const HIDDEN: &str = "hidden";
const SHOWN: &str = "shown";

/// A pane beside the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    /// The vault's notes, on the left.
    Sidebar,
    /// What is known about the page, on the right.
    Details,
}

impl Pane {
    /// Name of the pane's setting in `prefs`.
    fn setting(self) -> &'static str {
        match self {
            Self::Sidebar => "sidebar",
            Self::Details => "details",
        }
    }
}

/// Whether a pane whose setting holds `stored` is shown.
fn is_open(stored: Option<&str>) -> bool {
    stored.map(str::trim) != Some(HIDDEN)
}

/// Whether `pane` was left shown.
pub fn load(pane: Pane) -> bool {
    is_open(prefs::read(pane.setting()).as_deref())
}

/// Remember whether `pane` is shown.
pub fn store(pane: Pane, open: bool) {
    prefs::write(pane.setting(), if open { SHOWN } else { HIDDEN });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pane_is_shown_unless_it_was_hidden() {
        assert!(is_open(None));
        assert!(is_open(Some(SHOWN)));
        assert!(!is_open(Some(HIDDEN)));
        assert!(!is_open(Some(" hidden\n")));
    }

    #[test]
    fn nonsense_does_not_hide_a_pane() {
        for text in ["", "off", "false", "0", "Hidden"] {
            assert!(is_open(Some(text)), "{text}");
        }
    }

    #[test]
    fn each_pane_has_a_setting_of_its_own() {
        assert_ne!(Pane::Sidebar.setting(), Pane::Details.setting());
    }
}
