//! Short messages for the user: what to show and where it goes.
//!
//! A [`Notice`] is shown as a toast in the window's corner while the window is
//! on screen, and as a Windows notification while it is hidden or minimized.
//! Nothing here draws or touches GPUI's state: `main.rs` decides where a
//! notice goes and draws the stack (`mui::snackbar`), and this module keeps
//! what is shown and what a click on a Windows notification means.

use gpui_kit::{SystemNotification, SystemNotificationAction};

use crate::doc::Action;
use crate::mui::feedback::Tone;

/// The most toasts on screen at once. A new one beyond that pushes the oldest
/// out.
pub const MAX_SHOWN: usize = 3;

/// The most notices kept to answer a click on a Windows notification.
const MAX_SENT: usize = 8;

/// The id of the one button a Windows notification has.
const ACTION_ID: &str = "action";

/// One message for the user.
#[derive(Debug, Clone, PartialEq)]
pub struct Notice {
    /// Names the message: a new notice with the key of one still shown takes
    /// its place, and the key is the tag of the Windows notification.
    pub key: String,
    pub tone: Tone,
    pub title: String,
    pub body: Option<String>,
    /// A label, and what pressing it does.
    pub action: Option<(String, Action)>,
}

/// A notice on screen. `seq` tells it apart from an earlier one with the same
/// key.
#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub seq: u64,
    pub notice: Notice,
}

/// The toasts on screen, the ones that are leaving, and the notices sent to
/// Windows.
#[derive(Debug, Default)]
pub struct Toasts {
    next: u64,
    shown: Vec<Toast>,
    /// Taken off but still drawn, fading out. Kept to the same size as
    /// `shown`.
    leaving: Vec<Toast>,
    sent: Vec<Notice>,
}

impl Toasts {
    /// Show a notice and return its sequence number, for `expire`. One with
    /// the same key is replaced and the new one is the newest.
    pub fn push(&mut self, notice: Notice) -> u64 {
        if let Some(index) = self.shown.iter().position(|t| t.notice.key == notice.key) {
            // Replaced in place: the new one is drawn at once, with no fade.
            self.shown.remove(index);
        }
        self.next += 1;
        let seq = self.next;
        self.shown.push(Toast { seq, notice });
        while self.shown.len() > MAX_SHOWN {
            let oldest = self.shown.remove(0);
            self.leave(oldest);
        }
        seq
    }

    /// Take off the toast that still has this sequence number. Returns
    /// whether there was one: an older toast with the same key may have been
    /// replaced since, and its timer must not remove the newer one.
    pub fn expire(&mut self, seq: u64) -> bool {
        let Some(index) = self.shown.iter().position(|toast| toast.seq == seq) else {
            return false;
        };
        let toast = self.shown.remove(index);
        self.leave(toast);
        true
    }

    /// Take off the toast with this key. Returns whether there was one.
    pub fn dismiss(&mut self, key: &str) -> bool {
        let Some(index) = self.shown.iter().position(|toast| toast.notice.key == key) else {
            return false;
        };
        let toast = self.shown.remove(index);
        self.leave(toast);
        true
    }

    fn leave(&mut self, toast: Toast) {
        self.leaving.push(toast);
        if self.leaving.len() > MAX_SHOWN {
            self.leaving.remove(0);
        }
    }

    /// The toasts on screen, oldest first.
    pub fn shown(&self) -> &[Toast] {
        &self.shown
    }

    /// The toasts that were just taken off, which are still fading out.
    pub fn leaving(&self) -> &[Toast] {
        &self.leaving
    }

    /// Remember a notice that goes to Windows, so that a click on it can be
    /// answered by [`Toasts::route`].
    pub fn sent(&mut self, notice: &Notice) {
        self.sent.retain(|sent| sent.key != notice.key);
        self.sent.push(notice.clone());
        if self.sent.len() > MAX_SENT {
            self.sent.remove(0);
        }
    }

    /// What a click on a Windows notification does: the action of the notice
    /// with this tag, for a click on its button or on its body. `None` for a
    /// notice with no action, and for one this app does not know.
    pub fn route(&self, tag: &str, action_id: Option<&str>) -> Option<Action> {
        if action_id.is_some_and(|id| id != ACTION_ID) {
            return None;
        }
        let notice = self.sent.iter().find(|notice| notice.key == tag)?;
        notice.action.as_ref().map(|(_, action)| action.clone())
    }
}

/// The notice as a Windows notification.
pub fn system(notice: &Notice) -> SystemNotification {
    SystemNotification {
        tag: notice.key.clone().into(),
        title: notice.title.clone().into(),
        body: notice.body.clone().unwrap_or_default().into(),
        actions: notice
            .action
            .iter()
            .map(|(label, _)| SystemNotificationAction {
                id: ACTION_ID.into(),
                label: label.clone().into(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notice(key: &str) -> Notice {
        Notice {
            key: key.into(),
            tone: Tone::Info,
            title: format!("title of {key}"),
            body: None,
            action: None,
        }
    }

    fn with_action(key: &str) -> Notice {
        Notice {
            action: Some(("Open review".into(), Action::Note(format!("{key}.md")))),
            ..notice(key)
        }
    }

    fn keys(toasts: &Toasts) -> Vec<&str> {
        toasts
            .shown()
            .iter()
            .map(|toast| toast.notice.key.as_str())
            .collect()
    }

    #[test]
    fn a_pushed_notice_is_shown() {
        let mut toasts = Toasts::default();
        let seq = toasts.push(notice("a"));
        assert_eq!(keys(&toasts), ["a"]);
        assert_eq!(toasts.shown()[0].seq, seq);
    }

    #[test]
    fn the_newest_toast_is_last() {
        let mut toasts = Toasts::default();
        toasts.push(notice("a"));
        toasts.push(notice("b"));
        assert_eq!(keys(&toasts), ["a", "b"]);
    }

    #[test]
    fn a_notice_with_the_same_key_replaces_the_toast_and_gets_a_new_seq() {
        let mut toasts = Toasts::default();
        let first = toasts.push(notice("a"));
        toasts.push(notice("b"));
        let mut again = notice("a");
        again.title = "changed".into();
        let second = toasts.push(again);
        assert_ne!(first, second);
        // The replaced one is now the newest.
        assert_eq!(keys(&toasts), ["b", "a"]);
        assert_eq!(toasts.shown()[1].notice.title, "changed");
    }

    #[test]
    fn at_most_three_toasts_stay_and_the_oldest_goes() {
        let mut toasts = Toasts::default();
        for key in ["a", "b", "c", "d"] {
            toasts.push(notice(key));
        }
        assert_eq!(keys(&toasts), ["b", "c", "d"]);
        // The one pushed out fades; it does not vanish at once.
        assert_eq!(toasts.leaving().len(), 1);
        assert_eq!(toasts.leaving()[0].notice.key, "a");
    }

    #[test]
    fn expiring_removes_only_the_toast_that_still_has_the_seq() {
        let mut toasts = Toasts::default();
        let old = toasts.push(notice("a"));
        let new = toasts.push(notice("a"));
        // The first one's timer ends after the second replaced it.
        assert!(!toasts.expire(old));
        assert_eq!(keys(&toasts), ["a"]);
        assert!(toasts.expire(new));
        assert!(toasts.shown().is_empty());
        // And a second time there is nothing to remove.
        assert!(!toasts.expire(new));
    }

    #[test]
    fn dismissing_removes_the_toast_with_the_key() {
        let mut toasts = Toasts::default();
        toasts.push(notice("a"));
        toasts.push(notice("b"));
        assert!(toasts.dismiss("a"));
        assert_eq!(keys(&toasts), ["b"]);
        assert!(!toasts.dismiss("a"));
        assert_eq!(toasts.leaving()[0].notice.key, "a");
    }

    #[test]
    fn the_leaving_list_does_not_grow_without_end() {
        let mut toasts = Toasts::default();
        for n in 0..20 {
            let seq = toasts.push(notice(&n.to_string()));
            toasts.expire(seq);
        }
        assert!(toasts.leaving().len() <= MAX_SHOWN);
    }

    #[test]
    fn a_windows_notification_has_the_key_as_tag() {
        let mut notice = notice("a");
        notice.body = Some("more".into());
        let system = system(&notice);
        assert_eq!(system.tag.as_ref(), "a");
        assert_eq!(system.title.as_ref(), "title of a");
        assert_eq!(system.body.as_ref(), "more");
        assert!(system.actions.is_empty());
    }

    #[test]
    fn a_notice_without_a_body_has_an_empty_one() {
        assert_eq!(system(&notice("a")).body.as_ref(), "");
    }

    #[test]
    fn an_action_becomes_the_one_button() {
        let system = system(&with_action("a"));
        assert_eq!(system.actions.len(), 1);
        assert_eq!(system.actions[0].label.as_ref(), "Open review");
        assert_eq!(system.actions[0].id.as_ref(), ACTION_ID);
    }

    #[test]
    fn a_click_on_the_button_or_the_body_runs_the_action() {
        let mut toasts = Toasts::default();
        toasts.sent(&with_action("a"));
        let action = Some(Action::Note("a.md".into()));
        assert_eq!(toasts.route("a", Some(ACTION_ID)), action);
        assert_eq!(toasts.route("a", None), action);
        // A button this app never offered does nothing.
        assert_eq!(toasts.route("a", Some("other")), None);
    }

    #[test]
    fn a_notice_without_an_action_only_shows_the_window() {
        let mut toasts = Toasts::default();
        toasts.sent(&notice("a"));
        assert_eq!(toasts.route("a", None), None);
        assert_eq!(toasts.route("unknown", None), None);
    }

    #[test]
    fn a_later_notice_with_the_same_tag_answers_the_click() {
        let mut toasts = Toasts::default();
        toasts.sent(&with_action("a"));
        toasts.sent(&notice("a"));
        assert_eq!(toasts.route("a", None), None);
    }

    #[test]
    fn only_the_latest_eight_sent_notices_are_kept() {
        let mut toasts = Toasts::default();
        for n in 0..9 {
            toasts.sent(&with_action(&n.to_string()));
        }
        assert_eq!(toasts.route("0", None), None);
        assert!(toasts.route("1", None).is_some());
        assert!(toasts.route("8", None).is_some());
    }
}
