//! What the search field suggests while it is typed in: the hits of its text,
//! looked up at each pause in typing, in a list under the field.
//!
//! The field keeps the keys. Down moves from the field into the list and on
//! down it; Up moves back up and, from the first row, back to the field.
//! Enter opens the highlighted hit, or, with none highlighted, the Search
//! page of every hit, as before the list existed.

use std::time::Duration;

use gpui_kit::Task;
use serde_json::Value;

use crate::live::{self, HitTarget};
use crate::mui::menu::AnchorBounds;

/// How long typing must pause before the field's text is looked up.
pub const PAUSE: Duration = Duration::from_millis(150);
/// How many hits the list shows.
pub const LIMIT: usize = 10;

#[derive(Default)]
pub struct Suggestions {
    /// The text the hits are for, trimmed.
    query: String,
    groups: Vec<(&'static str, Vec<HitTarget>)>,
    open: bool,
    /// The row the keys act on; `None` while they act on the field.
    highlighted: Option<usize>,
    /// The lookup that waits for typing to pause, or runs. Dropping it
    /// cancels it, so a new one replaces it at each change of the text.
    pub task: Option<Task<()>>,
    /// Where the field is drawn, for the list to hang under.
    pub anchor: AnchorBounds,
}

impl Suggestions {
    /// Show the hits of `query`, with no row highlighted.
    pub fn show(&mut self, query: &str, hits: &Value) {
        self.query = query.trim().to_string();
        self.groups = live::hit_groups(hits);
        self.open = true;
        self.highlighted = None;
    }

    /// Hide the list, keeping its hits for when the field has the focus
    /// again, and drop a lookup that has not answered.
    pub fn close(&mut self) {
        self.open = false;
        self.highlighted = None;
        self.task = None;
    }

    /// Forget the hits: the field is empty.
    pub fn clear(&mut self) {
        self.close();
        self.query.clear();
        self.groups.clear();
    }

    /// The text changed: the keys act on the field again until new hits
    /// come. The old ones stay in view meanwhile.
    pub fn typed(&mut self) {
        self.highlighted = None;
    }

    /// Show the list again if its hits are those of `text`. Whether it is
    /// open now.
    pub fn reopen(&mut self, text: &str) -> bool {
        if !self.query.is_empty() && self.query == text.trim() {
            self.open = true;
        }
        self.open
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn groups(&self) -> &[(&'static str, Vec<HitTarget>)] {
        &self.groups
    }

    pub fn highlighted(&self) -> Option<usize> {
        self.highlighted
    }

    fn count(&self) -> usize {
        self.groups.iter().map(|(_, targets)| targets.len()).sum()
    }

    /// The hit at `index`, counted across the groups.
    pub fn target(&self, index: usize) -> Option<&HitTarget> {
        self.groups
            .iter()
            .flat_map(|(_, targets)| targets)
            .nth(index)
    }

    /// The highlighted hit, which Enter opens.
    pub fn chosen(&self) -> Option<&HitTarget> {
        self.target(self.highlighted?)
    }

    /// Down: into the list from the field, then on down it, stopping at the
    /// last row.
    pub fn down(&mut self) {
        let count = self.count();
        if count == 0 {
            return;
        }
        self.highlighted = Some(match self.highlighted {
            None => 0,
            Some(row) => (row + 1).min(count - 1),
        });
    }

    /// Up: back up the list, and from its first row back to the field.
    pub fn up(&mut self) {
        self.highlighted = match self.highlighted {
            None | Some(0) => None,
            Some(row) => Some(row - 1),
        };
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::doc::Action;

    fn shown() -> Suggestions {
        let mut suggestions = Suggestions::default();
        suggestions.show(
            " okx ",
            &json!([
                {"kind": "symbol", "key": "app::OKXClient"},
                {"kind": "note", "key": "a/OKX REST Client.md", "title": "OKX REST Client"},
                {"kind": "file", "key": "src/okx.py"}
            ]),
        );
        suggestions
    }

    #[test]
    fn rows_are_counted_across_the_groups_in_their_order() {
        let suggestions = shown();
        let headings: Vec<&str> = suggestions.groups().iter().map(|(h, _)| *h).collect();
        assert_eq!(headings, ["Notes", "Symbols", "Files"]);
        let actions: Vec<Action> = (0..3)
            .map(|row| suggestions.target(row).unwrap().action.clone())
            .collect();
        assert_eq!(
            actions,
            [
                Action::Note("a/OKX REST Client.md".into()),
                Action::Symbol("app::OKXClient".into()),
                Action::File("src/okx.py".into())
            ]
        );
        assert!(suggestions.target(3).is_none());
    }

    #[test]
    fn the_keys_move_from_the_field_into_the_list_and_back() {
        let mut suggestions = shown();
        assert!(suggestions.is_open());
        assert_eq!(
            suggestions.chosen(),
            None,
            "Enter searches the field's text"
        );
        suggestions.down();
        assert_eq!(suggestions.highlighted(), Some(0));
        suggestions.down();
        suggestions.down();
        suggestions.down();
        assert_eq!(
            suggestions.highlighted(),
            Some(2),
            "Down stops at the last row"
        );
        assert_eq!(
            suggestions.chosen().map(|t| t.action.clone()),
            Some(Action::File("src/okx.py".into()))
        );
        suggestions.up();
        suggestions.up();
        assert_eq!(suggestions.highlighted(), Some(0));
        suggestions.up();
        assert_eq!(suggestions.highlighted(), None, "Up from the first row");
        suggestions.down();
        suggestions.typed();
        assert_eq!(
            suggestions.highlighted(),
            None,
            "typing gives the keys back to the field"
        );
    }

    #[test]
    fn an_empty_list_has_nowhere_to_go() {
        let mut suggestions = Suggestions::default();
        suggestions.show("zzz", &json!([]));
        assert!(suggestions.is_open());
        suggestions.down();
        assert_eq!(suggestions.highlighted(), None);
    }

    #[test]
    fn the_list_comes_back_only_for_the_text_it_was_for() {
        let mut suggestions = shown();
        suggestions.down();
        suggestions.close();
        assert!(!suggestions.is_open());
        assert_eq!(suggestions.highlighted(), None);
        assert!(!suggestions.reopen("okx rest"));
        assert!(suggestions.reopen("okx"));
        suggestions.clear();
        assert!(!suggestions.reopen(""));
        assert!(suggestions.groups().is_empty());
    }
}
