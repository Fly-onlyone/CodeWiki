//! The review queue as the app keeps it: the pending proposals, which of them
//! are selected, which opens after a decision, and what the user is told.
//! No UI here; `review_view.rs` draws the page and runs the requests.
//!
//! A sync proposes a change to a note someone edited by hand, and a coding
//! agent proposes its explanation of such a note. Each waits for a decision,
//! one at a time or several at once.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use cw_client::DecideResult;
use serde_json::Value;

use crate::doc::Action;
use crate::mui::checkbox::Check;
use crate::mui::feedback::Tone;
use crate::toasts::Notice;

/// What made a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A sync: the code under the note changed.
    Sync,
    /// A coding agent that explained the note.
    Agent,
}

impl Kind {
    /// The word on the proposal's chip.
    pub fn label(self) -> &'static str {
        match self {
            Self::Sync => "Sync",
            Self::Agent => "Agent",
        }
    }
}

/// One pending proposal, as the queue lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: String,
    /// Vault-relative path of the note.
    pub note_path: String,
    pub kind: Kind,
    pub reason: String,
}

impl Item {
    /// The note's name: its file name without `.md`.
    pub fn name(&self) -> &str {
        note_name(&self.note_path)
    }
}

/// The name of the note at `path`, as the queue and the toasts say it.
pub fn note_name(path: &str) -> &str {
    let file = path.rsplit('/').next().unwrap_or(path);
    file.strip_suffix(".md").unwrap_or(file)
}

/// The pending proposals in what `GET /v1/repos/{id}/proposals` answered,
/// in its order. A proposal with no kind is a sync's, as on the backend.
pub fn pending(proposals: &Value) -> Vec<Item> {
    proposals
        .as_array()
        .into_iter()
        .flatten()
        .filter(|proposal| proposal["status"] == "pending")
        .filter_map(|proposal| {
            Some(Item {
                id: proposal["id"].as_str()?.to_string(),
                note_path: proposal["note_path"].as_str()?.to_string(),
                kind: if proposal["kind"] == "agent" {
                    Kind::Agent
                } else {
                    Kind::Sync
                },
                reason: proposal["reason"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect()
}

/// How a row of the queue was clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    /// The row alone.
    Plain,
    /// With Ctrl: the row joins the selection or leaves it.
    Toggle,
    /// With Shift: the rows from the last one clicked to this one.
    Range,
}

impl Click {
    /// The click the keys held make. Shift wins over Ctrl.
    pub fn with_keys(control: bool, shift: bool) -> Self {
        match (control, shift) {
            (_, true) => Self::Range,
            (true, false) => Self::Toggle,
            (false, false) => Self::Plain,
        }
    }
}

/// Which proposals of the queue are selected, by id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    ids: BTreeSet<String>,
    /// The row a range starts from: the last one clicked.
    anchor: Option<String>,
}

impl Selection {
    /// Change the selection as a click on the row `id` of the queue `order`
    /// does. Returns the proposal to open: a plain click opens its row.
    pub fn click(&mut self, order: &[String], id: &str, how: Click) -> Option<String> {
        let row = order.iter().position(|known| known == id)?;
        match how {
            Click::Plain => {
                self.ids = BTreeSet::from([id.to_string()]);
                self.anchor = Some(id.to_string());
                return Some(id.to_string());
            }
            Click::Toggle => self.toggle(id),
            Click::Range => {
                let from = self
                    .anchor
                    .as_ref()
                    .and_then(|anchor| order.iter().position(|known| known == anchor));
                match from {
                    Some(from) => {
                        let (low, high) = (from.min(row), from.max(row));
                        self.ids = order[low..=high].iter().cloned().collect();
                    }
                    None => {
                        self.ids = BTreeSet::from([id.to_string()]);
                        self.anchor = Some(id.to_string());
                    }
                }
            }
        }
        None
    }

    /// Add `id` to the selection, or take it out: its checkbox, or a click
    /// with Ctrl.
    pub fn toggle(&mut self, id: &str) {
        if !self.ids.remove(id) {
            self.ids.insert(id.to_string());
        }
        self.anchor = Some(id.to_string());
    }

    pub fn select_all(&mut self, order: &[String]) {
        self.ids = order.iter().cloned().collect();
    }

    pub fn clear(&mut self) {
        self.ids.clear();
        self.anchor = None;
    }

    /// The checkbox above the queue: everything selected clears the
    /// selection, anything else selects every row.
    pub fn toggle_all(&mut self, order: &[String]) {
        if self.header(order) == Check::Checked {
            self.clear();
        } else {
            self.select_all(order);
        }
    }

    /// Drop the ids that are no longer pending.
    pub fn retain(&mut self, pending: &[String]) {
        self.ids.retain(|id| pending.contains(id));
        if self
            .anchor
            .as_ref()
            .is_some_and(|anchor| !pending.contains(anchor))
        {
            self.anchor = None;
        }
    }

    pub fn contains(&self, id: &str) -> bool {
        self.ids.contains(id)
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// The selected ids, in the order of the queue.
    pub fn in_order(&self, order: &[String]) -> Vec<String> {
        order
            .iter()
            .filter(|id| self.ids.contains(*id))
            .cloned()
            .collect()
    }

    /// What the checkbox above the queue shows: every row selected, none, or
    /// some.
    pub fn header(&self, order: &[String]) -> Check {
        let selected = order.iter().filter(|id| self.ids.contains(*id)).count();
        match selected {
            0 => Check::Unchecked,
            n if n == order.len() => Check::Checked,
            _ => Check::Indeterminate,
        }
    }
}

/// The proposal to open after a decision on `decided`: the first one after
/// `current` in `order` that was not decided, else the nearest one before
/// it, else none. With no `current`, or one not in `order`, the first that
/// was not decided.
pub fn next_after(order: &[String], decided: &[String], current: Option<&str>) -> Option<String> {
    let left = |id: &&String| !decided.contains(*id);
    let at = current.and_then(|current| order.iter().position(|id| id == current));
    let Some(at) = at else {
        return order.iter().find(left).cloned();
    };
    order[at + 1..]
        .iter()
        .find(left)
        .or_else(|| order[..at].iter().rev().find(left))
        .cloned()
}

/// The proposal to show once the queue has been loaded again after a
/// decision. `order` is the queue before, `pending` after. The open one
/// stays when it was not decided and still waits; otherwise the next one
/// ([`next_after`]), and the first of the queue when that finds none. `None`
/// only when nothing waits.
pub fn after_decision(
    order: &[String],
    pending: &[String],
    decided: &[String],
    current: Option<&str>,
) -> Option<String> {
    if let Some(current) = current
        && pending.iter().any(|id| id == current)
        && !decided.iter().any(|id| id == current)
    {
        return Some(current.to_string());
    }
    // What another client decided meanwhile is gone as well.
    let gone: Vec<String> = order
        .iter()
        .filter(|id| decided.contains(*id) || !pending.contains(*id))
        .cloned()
        .collect();
    next_after(order, &gone, current).or_else(|| pending.first().cloned())
}

/// The warning shown while the selection holds a sync's and an agent's
/// proposal for the same note: approving both writes one, and the other no
/// longer fits the note. `None` when there is no such pair.
pub fn clash_warning(items: &[Item], selection: &Selection) -> Option<String> {
    // Each note with a selected proposal, and which kinds are selected.
    let mut notes: Vec<(&str, &str, [bool; 2])> = Vec::new();
    for item in items.iter().filter(|item| selection.contains(&item.id)) {
        let slot = match notes
            .iter()
            .position(|(path, _, _)| *path == item.note_path)
        {
            Some(slot) => slot,
            None => {
                notes.push((&item.note_path, item.name(), [false; 2]));
                notes.len() - 1
            }
        };
        notes[slot].2[usize::from(item.kind == Kind::Agent)] = true;
    }
    let names: Vec<&str> = notes
        .iter()
        .filter(|(_, _, [sync, agent])| *sync && *agent)
        .map(|(_, name, _)| *name)
        .collect();
    (!names.is_empty()).then(|| {
        format!(
            "{}: a sync and an agent proposal are both selected. Only one of them can be written, and the other will be superseded.",
            names.join(", ")
        )
    })
}

/// The lines of the details pane beside the queue.
pub fn counts(items: &[Item], selected: usize) -> Vec<String> {
    if items.is_empty() {
        return vec!["Nothing waits".to_string()];
    }
    let agents = items.iter().filter(|item| item.kind == Kind::Agent).count();
    let syncs = items.len() - agents;
    let mut lines = vec![format!("{} waiting", items.len())];
    if syncs > 0 {
        lines.push(format!("{syncs} from a sync"));
    }
    if agents > 0 {
        lines.push(format!("{agents} from a coding agent"));
    }
    if selected > 0 {
        lines.push(format!("{selected} selected"));
    }
    lines
}

/// What the user is told after a decision: what went through, and a warning
/// that lists what did not. `items` is the queue before the decision, for
/// the names of the notes.
pub fn decided_notices(approve: bool, results: &[DecideResult], items: &[Item]) -> Vec<Notice> {
    // The note a proposal was for: as the backend names it, else as the
    // queue did, else the proposal's id.
    let path = |result: &DecideResult| -> Option<String> {
        result.note_path.clone().or_else(|| {
            let item = items.iter().find(|item| item.id == result.id)?;
            Some(item.note_path.clone())
        })
    };
    let name = |result: &DecideResult| -> String {
        path(result).map_or_else(|| result.id.clone(), |path| note_name(&path).to_string())
    };
    let notice = |key: &str, tone, title: String| Notice {
        key: key.into(),
        tone,
        title,
        body: None,
        action: None,
    };
    let (done, failed): (Vec<&DecideResult>, Vec<&DecideResult>) =
        results.iter().partition(|result| result.ok);
    let mut notices = Vec::new();
    let title = match (done.as_slice(), approve) {
        ([], _) => None,
        ([one], true) => Some(format!("Written to {}", name(one))),
        ([one], false) => Some(format!("Rejected the proposal for {}", name(one))),
        (several, true) => Some(format!("Approved {}", several.len())),
        (several, false) => Some(format!("Rejected {}", several.len())),
    };
    if let Some(title) = title {
        // One note written: the way to it.
        let open = match done.as_slice() {
            [one] if approve => path(one).map(|path| ("Open note".to_string(), Action::Note(path))),
            _ => None,
        };
        notices.push(Notice {
            action: open,
            ..notice(DECIDED_KEY, Tone::Success, title)
        });
    }
    if !failed.is_empty() {
        let verb = if approve { "approved" } else { "rejected" };
        let title = match failed.len() {
            1 => format!("1 proposal was not {verb}"),
            n => format!("{n} proposals were not {verb}"),
        };
        let lines: Vec<String> = failed
            .iter()
            .map(|result| {
                let why = result
                    .error
                    .as_deref()
                    .unwrap_or("the backend did not say why");
                format!("{}: {why}", name(result))
            })
            .collect();
        notices.push(Notice {
            body: Some(lines.join("\n")),
            ..notice(FAILED_KEY, Tone::Warning, title)
        });
    }
    notices
}

/// The key of the notice of what a decision did, and of the one that lists
/// what it could not do; each replaces the last of its kind.
const DECIDED_KEY: &str = "review-decided";
const FAILED_KEY: &str = "review-failed";

/// The notice that new proposals wait in the repository `id`, named `name`.
pub fn rise_notice(id: &str, name: &str, increase: usize) -> Notice {
    let title = match increase {
        1 => format!("1 new review in {name}"),
        n => format!("{n} new reviews in {name}"),
    };
    Notice {
        key: format!("review:{id}"),
        tone: Tone::Info,
        title,
        body: None,
        action: Some(("Open review".into(), Action::OpenReview(id.to_string()))),
    }
}

/// What the buttons above the open proposal decide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Targets {
    /// Nothing that waits is open or selected.
    Nothing,
    /// The proposal on the page, alone.
    Open(String),
    /// The selected proposals, in the queue's order.
    Selected(Vec<String>),
}

impl Targets {
    /// The ids to decide.
    pub fn ids(&self) -> Vec<String> {
        match self {
            Self::Nothing => Vec::new(),
            Self::Open(id) => vec![id.clone()],
            Self::Selected(ids) => ids.clone(),
        }
    }

    /// The labels of the approve and the reject button; `None` when there
    /// is nothing to decide.
    pub fn labels(&self) -> Option<(String, String)> {
        match self {
            Self::Nothing => None,
            Self::Open(_) => Some(("Approve and write".into(), "Reject".into())),
            Self::Selected(ids) => Some((
                format!("Approve {}", ids.len()),
                format!("Reject {}", ids.len()),
            )),
        }
    }
}

/// What a decision acts on: the selection whenever it holds anything, the
/// open proposal only while nothing is selected. A selection of the open
/// proposal alone is that proposal, and so is a selection of the one whose
/// detail is `loading`, which a plain click on its row is about to open.
/// Only what waits in `order` counts.
pub fn targets(
    order: &[String],
    selection: &Selection,
    open: Option<&str>,
    loading: Option<&str>,
) -> Targets {
    let waits = |id: &&str| order.iter().any(|known| known == id);
    let open = open.filter(waits);
    match (selection.in_order(order).as_slice(), open) {
        ([], Some(open)) => Targets::Open(open.to_string()),
        ([], None) => Targets::Nothing,
        ([one], _) if Some(one.as_str()) == open || Some(one.as_str()) == loading => {
            Targets::Open(one.clone())
        }
        (selected, _) => Targets::Selected(selected.to_vec()),
    }
}

/// The notes an approval wrote, by vault path: none for a rejection.
pub fn written(approve: bool, results: &[DecideResult], items: &[Item]) -> Vec<String> {
    if !approve {
        return Vec::new();
    }
    results
        .iter()
        .filter(|result| result.ok)
        .filter_map(|result| {
            result.note_path.clone().or_else(|| {
                let item = items.iter().find(|item| item.id == result.id)?;
                Some(item.note_path.clone())
            })
        })
        .collect()
}

/// Drop from `history` every page of a note in `written`, which shows the
/// note as it was before: going back to one and editing it would save the
/// old text over the approved one. `note` gives the path of a page that is a
/// note, `None` for any other page, which stays.
pub fn forget_written<T>(
    history: &mut Vec<T>,
    written: &[String],
    note: impl Fn(&T) -> Option<&str>,
) {
    history.retain(|page| note(page).is_none_or(|path| !written.iter().any(|w| w == path)));
}

/// The notes an approval wrote while other tabs showed them. Such a tab
/// holds the note as it was, and an edit there would save that over the
/// approved text, so it loads the note again when it is next shown.
#[derive(Debug, Default)]
pub struct Stale {
    /// Each note's path, with how many tabs show it as it was.
    tabs: BTreeMap<String, usize>,
}

impl Stale {
    /// Remember each page in `shown` (a note's path, or `None` for any
    /// other page) that shows a note in `written`.
    pub fn mark<'a>(
        &mut self,
        shown: impl IntoIterator<Item = Option<&'a str>>,
        written: &[String],
    ) {
        for path in shown.into_iter().flatten() {
            if written.iter().any(|written| written == path) {
                *self.tabs.entry(path.to_string()).or_default() += 1;
            }
        }
    }

    /// Whether a tab just shown, whose page is the note at `path`, must
    /// load it again. Each tab marked asks once.
    pub fn take(&mut self, path: &str) -> bool {
        let Some(count) = self.tabs.get_mut(path) else {
            return false;
        };
        *count -= 1;
        if *count == 0 {
            self.tabs.remove(path);
        }
        true
    }
}

/// Whether the app itself is busy with work that may add proposals to the
/// open repository: its own sync, and the loading of the open repository's
/// notes. `marked` says that the work running now is such work; a report
/// that loads is not. Counts that rise meanwhile are the app's own doing.
pub fn own_work(marked: bool, busy: Option<&str>) -> bool {
    marked && busy.is_some()
}

/// The repositories to tell the user about: those whose count rose between
/// `before` and `after` ([`crate::status::rises`]), less the open one while
/// the app's own work runs, since the app knows of those proposals itself.
/// Each is its id, its new count and by how much it rose.
pub fn news(
    before: Option<&HashMap<String, usize>>,
    after: &HashMap<String, usize>,
    open: Option<&str>,
    own_work: bool,
) -> Vec<(String, usize, usize)> {
    crate::status::rises(before, after)
        .into_iter()
        .filter(|(id, _, _)| !(own_work && open == Some(id.as_str())))
        .collect()
}

/// What the app keeps of the review page and of the counts beside it.
#[derive(Debug, Default)]
pub struct State {
    pub selection: Selection,
    /// The proposal whose detail was asked for last; a detail that arrives
    /// for another is dropped.
    pub wanted: Option<String>,
    /// How many proposals wait in each repository, as `GET /v1/status` last
    /// said; `None` before the first answer and from an older backend.
    pub counts: Option<HashMap<String, usize>>,
    /// Whether the app's own work ran when the status was last asked for.
    pub asked_during_own_work: bool,
    /// Whether the work the app runs now is its own sync or the loading of
    /// the open repository's notes ([`own_work`]).
    pub own_work: bool,
    /// The error a proposal that did not load left on the page; the next
    /// proposal that loads takes it away.
    pub load_error: Option<String>,
    /// The notes other tabs show as they were before an approval.
    pub stale: Stale,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn ids(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    fn item(id: &str, note: &str, kind: Kind) -> Item {
        Item {
            id: id.into(),
            note_path: format!("01-Core/{note}.md"),
            kind,
            reason: "source changed".into(),
        }
    }

    fn ok(id: &str, note: &str) -> DecideResult {
        DecideResult {
            id: id.into(),
            ok: true,
            note_path: Some(format!("01-Core/{note}.md")),
            error: None,
        }
    }

    fn failed(id: &str, error: &str) -> DecideResult {
        DecideResult {
            id: id.into(),
            ok: false,
            note_path: None,
            error: Some(error.into()),
        }
    }

    #[test]
    fn the_queue_lists_pending_proposals_in_the_backends_order() {
        let queue = pending(&json!([
            {"id": "parser-1", "note_path": "01-Core/Parser.md", "status": "pending",
             "kind": "sync", "reason": "source changed"},
            {"id": "lexer-1", "note_path": "01-Core/Lexer.md", "status": "approved"},
            {"id": "lexer-agent-2", "note_path": "01-Core/Lexer.md", "status": "pending",
             "kind": "agent", "reason": "claude-code explained this note"},
            // From before proposals had a kind: a sync's.
            {"id": "old-3", "note_path": "Old.md", "status": "pending"},
            {"note_path": "NoId.md", "status": "pending"},
        ]));
        assert_eq!(
            queue,
            vec![
                Item {
                    id: "parser-1".into(),
                    note_path: "01-Core/Parser.md".into(),
                    kind: Kind::Sync,
                    reason: "source changed".into(),
                },
                Item {
                    id: "lexer-agent-2".into(),
                    note_path: "01-Core/Lexer.md".into(),
                    kind: Kind::Agent,
                    reason: "claude-code explained this note".into(),
                },
                Item {
                    id: "old-3".into(),
                    note_path: "Old.md".into(),
                    kind: Kind::Sync,
                    reason: String::new(),
                },
            ]
        );
        assert!(pending(&json!("nonsense")).is_empty());
        assert_eq!(queue[0].name(), "Parser");
        assert_eq!(note_name("Old.md"), "Old");
        assert_eq!(note_name("a/b/c"), "c");
        assert_eq!((Kind::Sync.label(), Kind::Agent.label()), ("Sync", "Agent"));
    }

    #[test]
    fn a_plain_click_selects_its_row_alone_and_opens_it() {
        let order = ids(&["a", "b", "c"]);
        let mut selection = Selection::default();
        selection.select_all(&order);
        assert_eq!(
            selection.click(&order, "b", Click::Plain),
            Some("b".to_string())
        );
        assert_eq!(selection.in_order(&order), ids(&["b"]));
        // A row the queue does not hold is not selected.
        assert_eq!(selection.click(&order, "z", Click::Plain), None);
        assert_eq!(selection.in_order(&order), ids(&["b"]));
    }

    #[test]
    fn ctrl_click_and_the_checkbox_toggle_a_row() {
        let order = ids(&["a", "b", "c"]);
        let mut selection = Selection::default();
        selection.click(&order, "a", Click::Plain);
        assert_eq!(selection.click(&order, "c", Click::Toggle), None);
        assert_eq!(selection.in_order(&order), ids(&["a", "c"]));
        selection.click(&order, "a", Click::Toggle);
        assert_eq!(selection.in_order(&order), ids(&["c"]));
        selection.toggle("b");
        assert_eq!(selection.in_order(&order), ids(&["b", "c"]));
        selection.toggle("b");
        assert!(!selection.contains("b"));
        assert_eq!(selection.len(), 1);
    }

    #[test]
    fn shift_click_selects_the_range_from_the_anchor() {
        let order = ids(&["a", "b", "c", "d", "e"]);
        let mut selection = Selection::default();
        selection.click(&order, "b", Click::Plain);
        assert_eq!(selection.click(&order, "d", Click::Range), None);
        assert_eq!(selection.in_order(&order), ids(&["b", "c", "d"]));
        // The anchor stays: a second range from it replaces the first.
        selection.click(&order, "a", Click::Range);
        assert_eq!(selection.in_order(&order), ids(&["a", "b"]));
        // A toggled row is the new anchor.
        selection.click(&order, "e", Click::Toggle);
        selection.click(&order, "c", Click::Range);
        assert_eq!(selection.in_order(&order), ids(&["c", "d", "e"]));
        // With no anchor, a range is the row alone.
        let mut fresh = Selection::default();
        fresh.click(&order, "c", Click::Range);
        assert_eq!(fresh.in_order(&order), ids(&["c"]));
        assert_eq!(Click::with_keys(false, false), Click::Plain);
        assert_eq!(Click::with_keys(true, false), Click::Toggle);
        assert_eq!(Click::with_keys(false, true), Click::Range);
        assert_eq!(Click::with_keys(true, true), Click::Range);
    }

    #[test]
    fn the_header_checkbox_shows_all_none_or_some_and_toggles_all() {
        let order = ids(&["a", "b", "c"]);
        let mut selection = Selection::default();
        assert_eq!(selection.header(&order), Check::Unchecked);
        selection.toggle("b");
        assert_eq!(selection.header(&order), Check::Indeterminate);
        selection.toggle_all(&order);
        assert_eq!(selection.header(&order), Check::Checked);
        assert_eq!(selection.len(), 3);
        selection.toggle_all(&order);
        assert_eq!(selection.header(&order), Check::Unchecked);
        selection.select_all(&order);
        selection.clear();
        assert_eq!(selection.len(), 0);
        // An empty queue has nothing to select.
        assert_eq!(selection.header(&[]), Check::Unchecked);
    }

    #[test]
    fn retain_drops_what_no_longer_waits() {
        let order = ids(&["a", "b", "c"]);
        let mut selection = Selection::default();
        selection.click(&order, "a", Click::Plain);
        selection.click(&order, "c", Click::Range);
        selection.retain(&ids(&["b", "d"]));
        assert_eq!(selection.in_order(&ids(&["b", "d"])), ids(&["b"]));
        // The anchor went with its row: a range starts afresh.
        let now = ids(&["b", "d"]);
        selection.click(&now, "d", Click::Range);
        assert_eq!(selection.in_order(&now), ids(&["d"]));
    }

    #[test]
    fn the_next_proposal_is_after_the_current_else_before_it() {
        let order = ids(&["a", "b", "c", "d"]);
        assert_eq!(
            next_after(&order, &ids(&["b"]), Some("b")),
            Some("c".into())
        );
        assert_eq!(
            next_after(&order, &ids(&["b", "c"]), Some("b")),
            Some("d".into())
        );
        // Nothing after it: the nearest before.
        assert_eq!(
            next_after(&order, &ids(&["c", "d"]), Some("d")),
            Some("b".into())
        );
        assert_eq!(next_after(&order, &order, Some("b")), None);
        // No current one, or one the queue did not hold: the first left.
        assert_eq!(next_after(&order, &ids(&["a"]), None), Some("b".into()));
        assert_eq!(
            next_after(&order, &ids(&["a"]), Some("z")),
            Some("b".into())
        );
    }

    #[test]
    fn after_a_decision_the_open_one_stays_or_the_next_opens() {
        let order = ids(&["a", "b", "c"]);
        // The open one was decided: the next.
        assert_eq!(
            after_decision(&order, &ids(&["a", "c"]), &ids(&["b"]), Some("b")),
            Some("c".into())
        );
        // Several others were decided: the open one stays.
        assert_eq!(
            after_decision(&order, &ids(&["a"]), &ids(&["b", "c"]), Some("a")),
            Some("a".into())
        );
        // A proposal gone for another reason is skipped too.
        assert_eq!(
            after_decision(&order, &ids(&["a"]), &ids(&["b"]), Some("b")),
            Some("a".into())
        );
        // A decision that failed leaves its proposal pending; the next opens.
        assert_eq!(
            after_decision(&order, &ids(&["a", "b", "c"]), &ids(&["b"]), Some("b")),
            Some("c".into())
        );
        // Only a new proposal is left: it opens.
        assert_eq!(
            after_decision(&order, &ids(&["n"]), &order, Some("a")),
            Some("n".into())
        );
        // Nothing waits.
        assert_eq!(after_decision(&order, &[], &ids(&["a"]), Some("a")), None);
    }

    #[test]
    fn a_sync_and_an_agent_proposal_for_one_note_warn() {
        let items = [
            item("p-sync", "Parser", Kind::Sync),
            item("p-agent", "Parser", Kind::Agent),
            item("l-sync", "Lexer", Kind::Sync),
        ];
        let order: Vec<String> = items.iter().map(|item| item.id.clone()).collect();
        let mut selection = Selection::default();
        selection.select_all(&order);
        assert_eq!(
            clash_warning(&items, &selection).as_deref(),
            Some(
                "Parser: a sync and an agent proposal are both selected. Only one of them can be written, and the other will be superseded."
            )
        );
        selection.toggle("p-agent");
        assert_eq!(clash_warning(&items, &selection), None);
        let mut two_kinds_two_notes = Selection::default();
        two_kinds_two_notes.toggle("p-agent");
        two_kinds_two_notes.toggle("l-sync");
        assert_eq!(clash_warning(&items, &two_kinds_two_notes), None);
    }

    #[test]
    fn the_details_count_what_waits() {
        let items = [
            item("a", "A", Kind::Sync),
            item("b", "B", Kind::Sync),
            item("c", "C", Kind::Agent),
        ];
        assert_eq!(
            counts(&items, 2),
            [
                "3 waiting",
                "2 from a sync",
                "1 from a coding agent",
                "2 selected"
            ]
        );
        assert_eq!(counts(&items[..1], 0), ["1 waiting", "1 from a sync"]);
        assert_eq!(counts(&[], 0), ["Nothing waits"]);
    }

    #[test]
    fn one_approval_offers_its_note_and_several_are_counted() {
        let items = [
            item("a", "Parser", Kind::Sync),
            item("b", "Lexer", Kind::Agent),
        ];
        let one = decided_notices(true, &[ok("a", "Parser")], &items);
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].tone, Tone::Success);
        assert_eq!(one[0].title, "Written to Parser");
        assert_eq!(
            one[0].action,
            Some(("Open note".into(), Action::Note("01-Core/Parser.md".into())))
        );
        let several = decided_notices(true, &[ok("a", "Parser"), ok("b", "Lexer")], &items);
        assert_eq!(several.len(), 1);
        assert_eq!(several[0].title, "Approved 2");
        assert_eq!(several[0].action, None);
        let rejected = decided_notices(false, &[ok("a", "Parser")], &items);
        assert_eq!(rejected[0].title, "Rejected the proposal for Parser");
        assert_eq!(rejected[0].action, None);
        let rejected = decided_notices(false, &[ok("a", "Parser"), ok("b", "Lexer")], &items);
        assert_eq!(rejected[0].title, "Rejected 2");
    }

    #[test]
    fn what_did_not_go_through_is_listed_in_a_warning() {
        let items = [
            item("a", "Parser", Kind::Sync),
            item("b", "Lexer", Kind::Agent),
        ];
        let mixed = decided_notices(
            true,
            &[
                ok("a", "Parser"),
                failed("b", "proposal b changed since it was proposed"),
            ],
            &items,
        );
        assert_eq!(mixed.len(), 2);
        assert_eq!(mixed[0].title, "Written to Parser");
        assert_eq!(mixed[1].tone, Tone::Warning);
        assert_eq!(mixed[1].title, "1 proposal was not approved");
        assert_eq!(
            mixed[1].body.as_deref(),
            Some("Lexer: proposal b changed since it was proposed")
        );
        assert_ne!(mixed[0].key, mixed[1].key);
        let none = decided_notices(
            false,
            &[failed("a", "not pending"), failed("z", "no such proposal")],
            &items,
        );
        assert_eq!(none.len(), 1);
        assert_eq!(none[0].title, "2 proposals were not rejected");
        // A proposal the queue did not know is named by its id.
        assert_eq!(
            none[0].body.as_deref(),
            Some("Parser: not pending\nz: no such proposal")
        );
    }

    #[test]
    fn new_proposals_are_told_with_a_way_to_the_review() {
        let one = rise_notice("r1", "alpha", 1);
        assert_eq!(one.key, "review:r1");
        assert_eq!(one.title, "1 new review in alpha");
        assert_eq!(
            one.action,
            Some(("Open review".into(), Action::OpenReview("r1".into())))
        );
        assert_eq!(
            rise_notice("r1", "alpha", 3).title,
            "3 new reviews in alpha"
        );
    }

    #[test]
    fn with_nothing_selected_the_buttons_decide_the_open_proposal() {
        let order = ids(&["a", "b", "c"]);
        let none = Selection::default();
        assert_eq!(
            targets(&order, &none, Some("b"), None),
            Targets::Open("b".into())
        );
        assert_eq!(
            Targets::Open("b".into()).labels(),
            Some(("Approve and write".to_string(), "Reject".to_string()))
        );
        assert_eq!(Targets::Open("b".into()).ids(), ids(&["b"]));
        // An open proposal that no longer waits, or none: nothing to decide.
        assert_eq!(targets(&order, &none, Some("z"), None), Targets::Nothing);
        assert_eq!(targets(&order, &none, None, None), Targets::Nothing);
        assert_eq!(Targets::Nothing.labels(), None);
        assert!(Targets::Nothing.ids().is_empty());
    }

    #[test]
    fn a_selection_is_decided_and_never_the_open_proposal_beside_it() {
        let order = ids(&["a", "b", "c"]);
        // The open proposal selected alone is that proposal.
        let mut open_alone = Selection::default();
        open_alone.click(&order, "b", Click::Plain);
        assert_eq!(
            targets(&order, &open_alone, Some("b"), None),
            Targets::Open("b".into())
        );
        // One selected that is not the open one: that one, by count.
        let mut other = Selection::default();
        other.toggle("c");
        let one = targets(&order, &other, Some("b"), None);
        assert_eq!(one, Targets::Selected(ids(&["c"])));
        assert_eq!(one.ids(), ids(&["c"]));
        assert_eq!(
            one.labels(),
            Some(("Approve 1".to_string(), "Reject 1".to_string()))
        );
        // The same with nothing open, as after a detail that did not load.
        assert_eq!(
            targets(&order, &other, None, None),
            Targets::Selected(ids(&["c"]))
        );
        // Several, in the queue's order, whether or not the open one is
        // among them.
        let mut several = Selection::default();
        several.toggle("c");
        several.toggle("a");
        let both = targets(&order, &several, Some("b"), None);
        assert_eq!(both, Targets::Selected(ids(&["a", "c"])));
        assert_eq!(
            both.labels(),
            Some(("Approve 2".to_string(), "Reject 2".to_string()))
        );
        several.toggle("b");
        assert_eq!(
            targets(&order, &several, Some("b"), None),
            Targets::Selected(ids(&["a", "b", "c"]))
        );
        // What no longer waits is not decided.
        let gone = ids(&["a"]);
        assert_eq!(targets(&gone, &other, Some("b"), None), Targets::Nothing);
    }

    #[test]
    fn the_row_whose_detail_loads_counts_as_the_open_one() {
        let order = ids(&["a", "b", "c"]);
        // A plain click on c while b is open: c is selected alone, and its
        // detail is on its way.
        let mut clicked = Selection::default();
        clicked.click(&order, "c", Click::Plain);
        let loading = targets(&order, &clicked, Some("b"), Some("c"));
        assert_eq!(loading, Targets::Open("c".into()));
        assert_eq!(
            loading.labels(),
            Some(("Approve and write".to_string(), "Reject".to_string()))
        );
        // Once it failed to load, nothing loads: the checked row, by count.
        assert_eq!(
            targets(&order, &clicked, Some("b"), None),
            Targets::Selected(ids(&["c"]))
        );
        // Another row loading changes nothing for a checked one.
        assert_eq!(
            targets(&order, &clicked, Some("b"), Some("a")),
            Targets::Selected(ids(&["c"]))
        );
        // Several selected stay several.
        clicked.toggle("a");
        assert_eq!(
            targets(&order, &clicked, Some("b"), Some("c")),
            Targets::Selected(ids(&["a", "c"]))
        );
    }

    #[test]
    fn a_tab_that_showed_a_written_note_loads_it_once_when_shown() {
        let mut stale = Stale::default();
        let written = ids(&["01-Core/Parser.md", "01-Core/Eval.md"]);
        // Two tabs show Parser, one another page, one a note not written.
        stale.mark(
            [
                Some("01-Core/Parser.md"),
                None,
                Some("01-Core/Lexer.md"),
                Some("01-Core/Parser.md"),
            ],
            &written,
        );
        assert!(stale.take("01-Core/Parser.md"));
        assert!(stale.take("01-Core/Parser.md"));
        // Each tab asks once.
        assert!(!stale.take("01-Core/Parser.md"));
        assert!(!stale.take("01-Core/Lexer.md"));
        // A written note no tab showed is not waited for.
        assert!(!stale.take("01-Core/Eval.md"));
    }

    #[test]
    fn an_approval_names_the_notes_it_wrote() {
        let items = [
            item("a", "Parser", Kind::Sync),
            item("b", "Lexer", Kind::Agent),
        ];
        let results = [
            ok("a", "Parser"),
            failed("b", "changed since"),
            // A backend that did not name the note: the queue's path.
            DecideResult {
                note_path: None,
                ..ok("b", "Lexer")
            },
        ];
        assert_eq!(
            written(true, &results, &items),
            ["01-Core/Parser.md", "01-Core/Lexer.md"]
        );
        // A rejection writes nothing.
        assert!(written(false, &results, &items).is_empty());
    }

    #[test]
    fn the_history_forgets_the_notes_an_approval_wrote() {
        // A page: a note's path, or another page.
        let mut history = vec![
            Some("01-Core/Parser.md"),
            None,
            Some("01-Core/Lexer.md"),
            Some("01-Core/Parser.md"),
        ];
        forget_written(&mut history, &ids(&["01-Core/Parser.md"]), |page| *page);
        assert_eq!(history, vec![None, Some("01-Core/Lexer.md")]);
        forget_written(&mut history, &[], |page| *page);
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn the_apps_own_sync_is_no_news_in_the_open_repository() {
        // Marked work: the app's sync, the loading of the open repository.
        assert!(own_work(true, Some("Syncing wiki")));
        assert!(own_work(true, Some("Loading")));
        // A report loads under the same word, and is no such work.
        assert!(!own_work(false, Some("Loading")));
        assert!(!own_work(false, Some("Syncing wiki")));
        // A mark left from work that ended counts for nothing.
        assert!(!own_work(true, None));
        assert!(!own_work(false, None));
        let map = |pairs: &[(&str, usize)]| -> HashMap<String, usize> {
            pairs.iter().map(|(id, n)| (id.to_string(), *n)).collect()
        };
        let before = map(&[("open", 1), ("other", 0)]);
        let after = map(&[("open", 3), ("other", 2)]);
        assert_eq!(
            news(Some(&before), &after, Some("open"), false),
            vec![("open".to_string(), 3, 2), ("other".to_string(), 2, 2)]
        );
        assert_eq!(
            news(Some(&before), &after, Some("open"), true),
            vec![("other".to_string(), 2, 2)]
        );
        // The first answer is the baseline.
        assert!(news(None, &after, Some("open"), false).is_empty());
    }
}
