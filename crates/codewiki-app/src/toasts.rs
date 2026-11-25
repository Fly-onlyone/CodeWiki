//! Short messages for the user: what to show and where it goes.
//!
//! A [`Notice`] is shown as a toast in the window's corner while the window is
//! on screen, and as a Windows notification while it is hidden or minimized.
//! Nothing here draws or touches GPUI's state: `main.rs` decides where a
//! notice goes and draws the stack (`mui::snackbar`), and this module keeps
//! what is shown and what a click on a Windows notification means.

use std::collections::HashMap;
use std::time::{Duration, Instant};

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

/// The least time between two Windows notifications with one key, and how
/// long one counts as live. Without it, proposals that keep coming would
/// show a notification every 5 s, at each status poll.
pub const SYSTEM_GAP: Duration = Duration::from_secs(60);

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

/// What becomes of a notice offered to Windows ([`Toasts::offer`]).
#[derive(Debug, Clone, PartialEq)]
pub enum Offer {
    /// Show it now.
    Now(Notice),
    /// It waits until [`SYSTEM_GAP`] has passed since the last notification
    /// with its key, in place of any that waited with that key. Give it back
    /// after the wait, when one is given ([`Toasts::take_due`]); without one
    /// it took the place of a notice that already waits.
    Later(Option<Duration>),
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
    /// When a Windows notification with each key was last shown, for the
    /// last [`SYSTEM_GAP`].
    shown_at: HashMap<String, Instant>,
    /// The notices that wait for their gap, each with when it ends.
    waiting: Vec<(Notice, Instant)>,
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

    /// Offer a notice to Windows at `now`: it is shown at once, unless one
    /// with its key was shown less than [`SYSTEM_GAP`] ago; then it waits.
    pub fn offer(&mut self, notice: Notice, now: Instant) -> Offer {
        if let Some(waiting) = self.waiting.iter_mut().find(|(w, _)| w.key == notice.key) {
            waiting.0 = notice;
            return Offer::Later(None);
        }
        self.shown_at.retain(|_, at| now < *at + SYSTEM_GAP);
        if let Some(&at) = self.shown_at.get(&notice.key) {
            let due = at + SYSTEM_GAP;
            self.waiting.push((notice, due));
            return Offer::Later(Some(due - now));
        }
        self.shown_at.insert(notice.key.clone(), now);
        self.sent(&notice);
        Offer::Now(notice)
    }

    /// The notices whose wait is over at `now`, in the order they began to
    /// wait. Each is told again, as a toast or to Windows.
    pub fn take_due(&mut self, now: Instant) -> Vec<Notice> {
        let (due, waiting) = std::mem::take(&mut self.waiting)
            .into_iter()
            .partition(|(_, due)| *due <= now);
        self.waiting = waiting;
        due.into_iter().map(|(notice, _)| notice).collect()
    }

    /// How long until the next waiting notice is due; `None` when none waits.
    pub fn next_due(&self, now: Instant) -> Option<Duration> {
        self.waiting
            .iter()
            .map(|(_, due)| due.saturating_duration_since(now))
            .min()
    }

    /// Whether a notice with this key is still before the user: a toast on
    /// screen, or a Windows notification shown in the last [`SYSTEM_GAP`] or
    /// waiting to be.
    pub fn live(&self, key: &str, now: Instant) -> bool {
        self.shown.iter().any(|toast| toast.notice.key == key)
            || self.waiting.iter().any(|(notice, _)| notice.key == key)
            || self
                .shown_at
                .get(key)
                .is_some_and(|at| now < *at + SYSTEM_GAP)
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

    fn titled(key: &str, title: &str) -> Notice {
        Notice {
            title: title.into(),
            ..notice(key)
        }
    }

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn a_windows_notification_waits_a_minute_after_the_last_with_its_key() {
        let mut toasts = Toasts::default();
        let t0 = Instant::now();
        assert_eq!(
            toasts.offer(titled("a", "1"), t0),
            Offer::Now(titled("a", "1"))
        );
        // The next one with the key waits for the rest of the minute.
        assert_eq!(
            toasts.offer(titled("a", "2"), t0 + secs(5)),
            Offer::Later(Some(secs(55)))
        );
        assert_eq!(toasts.next_due(t0 + secs(5)), Some(secs(55)));
        // A newer one takes the place of the one that waits.
        assert_eq!(
            toasts.offer(titled("a", "3"), t0 + secs(10)),
            Offer::Later(None)
        );
        // Another key does not wait.
        assert_eq!(
            toasts.offer(notice("b"), t0 + secs(10)),
            Offer::Now(notice("b"))
        );
        assert!(toasts.take_due(t0 + secs(59)).is_empty());
        assert_eq!(toasts.take_due(t0 + secs(60)), vec![titled("a", "3")]);
        assert!(toasts.take_due(t0 + secs(61)).is_empty());
        assert_eq!(toasts.next_due(t0 + secs(61)), None);
        // Given back after the wait, it is shown.
        assert_eq!(
            toasts.offer(titled("a", "3"), t0 + secs(60)),
            Offer::Now(titled("a", "3"))
        );
    }

    #[test]
    fn an_offered_notice_answers_a_click() {
        let mut toasts = Toasts::default();
        toasts.offer(with_action("a"), Instant::now());
        assert!(toasts.route("a", None).is_some());
    }

    #[test]
    fn a_notice_is_live_while_its_toast_shows_or_for_a_minute_in_windows() {
        let mut toasts = Toasts::default();
        let t0 = Instant::now();
        assert!(!toasts.live("a", t0));
        let seq = toasts.push(notice("a"));
        assert!(toasts.live("a", t0));
        toasts.expire(seq);
        assert!(!toasts.live("a", t0));

        toasts.offer(notice("a"), t0);
        assert!(toasts.live("a", t0 + secs(59)));
        assert!(!toasts.live("a", t0 + secs(60)));
        assert!(!toasts.live("b", t0));
        // One that waits is live too.
        toasts.offer(notice("a"), t0 + secs(30));
        assert!(toasts.live("a", t0 + secs(61)));
        toasts.take_due(t0 + secs(61));
        assert!(!toasts.live("a", t0 + secs(61)));
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
