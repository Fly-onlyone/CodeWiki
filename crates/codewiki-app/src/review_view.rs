//! The review page as the app's view draws and runs it.
//!
//! The queue of pending proposals is at the left, the open proposal at the
//! right, drawn by the cached page view (`doc_element`), and above it the
//! buttons that decide. A click on a row shows its proposal in place, with
//! no history entry. One proposal is decided by its own routes, which every
//! backend has; several go to the backend in one call. After a decision the
//! queue loads again quietly and the next proposal opens.
//!
//! The status poll counts what waits in every repository; a count that rises
//! is news (`take_proposal_counts`). What is selected, what opens next and
//! what the user is told is worked out in `review.rs`.

use std::collections::HashMap;
use std::rc::Rc;

use cw_client::{Backend, DecideResult};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::*;
use serde_json::json;

use crate::doc::Doc;
use crate::mui::button::Button;
use crate::mui::checkbox::{Check, Checkbox};
use crate::mui::feedback::{self, Tone};
use crate::mui::list::{self, ListItemButton};
use crate::mui::surfaces;
use crate::mui::tokens::Variant;
use crate::review::{self, Click, Kind};
use crate::zoom::px;
use crate::{AppView, Page, glass, live};

/// Width of the queue beside the open proposal.
pub const QUEUE_WIDTH: f32 = 300.0;
/// The row above the queue, with the checkbox that selects every row:
/// `ListSubheader.js`, `lineHeight: '48px'`.
const QUEUE_HEAD: f32 = 48.0;

/// Where the review page shows a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Show {
    /// As a new page, with the page left kept in the history or not.
    Page { remember: bool },
    /// In place of what the review page shows, with no history entry, and
    /// only while the user stays on it.
    InPlace,
}

/// The review page when nothing waits.
fn empty_review() -> Page {
    Page::Review {
        open: None,
        doc: Rc::new(live::review_empty_page()),
    }
}

impl AppView {
    /// The ids of the queue, in its order.
    pub(crate) fn review_order(&self) -> Vec<String> {
        self.pending_proposals
            .iter()
            .map(|item| item.id.clone())
            .collect()
    }

    /// How much of the centre pane the queue takes: none on another page,
    /// and none when nothing waits.
    pub(crate) fn review_queue_width(&self) -> f32 {
        if matches!(self.page, Page::Review { .. }) && !self.pending_proposals.is_empty() {
            QUEUE_WIDTH
        } else {
            0.0
        }
    }

    /// The proposal open on the review page.
    fn review_open(&self) -> Option<&str> {
        match &self.page {
            Page::Review { open, .. } => open.as_deref(),
            _ => None,
        }
    }

    /// Open the review page of the open repository, as the user asked. On
    /// the page already, nothing changes.
    pub(crate) fn open_review(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.page, Page::Review { .. }) {
            self.show_review(Show::Page { remember: true }, cx);
        }
    }

    /// Show the review page on its first proposal, which is selected alone,
    /// or that nothing waits.
    pub(crate) fn show_review(&mut self, show: Show, cx: &mut Context<Self>) {
        if self.repo.is_none() {
            return;
        }
        let order = self.review_order();
        match (order.first().cloned(), show) {
            (Some(first), _) => {
                self.review.selection.click(&order, &first, Click::Plain);
                self.show_proposal(first, show, cx);
            }
            (None, Show::Page { remember }) => {
                self.review.selection.clear();
                self.set_page(empty_review(), remember);
                cx.notify();
            }
            (None, Show::InPlace) if matches!(self.page, Page::Review { .. }) => {
                self.set_page(empty_review(), false);
                cx.notify();
            }
            (None, Show::InPlace) => {}
        }
    }

    /// Open the review page of the repository `id`, as a click on a notice
    /// asked: at once when it is the open one, else once it has loaded.
    pub(crate) fn open_review_of(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.backend.is_none() {
            return;
        }
        if self.repo.as_ref().is_some_and(|repo| repo.id == id) {
            self.open_review(cx);
            return;
        }
        if let Some(repo) = self.repos.iter().find(|repo| repo.id == id).cloned() {
            self.select_repo(repo, crate::Landing::Review, cx);
        }
    }

    /// Show proposal `id` on the review page, as `show` says. It loads
    /// quietly, with no progress bar, so the queue does not flicker; only the
    /// last one asked for is shown.
    fn show_proposal(&mut self, id: String, show: Show, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        self.review.wanted = Some(id.clone());
        let route = format!("/v1/repos/{}/proposals/{id}", repo.id);
        self.settings_request(
            cx,
            move |b| b.get(&route),
            move |this, result, _| {
                if this.review.wanted.as_deref() != Some(id.as_str())
                    || this.repo.as_ref().is_none_or(|open| open.id != repo.id)
                {
                    return;
                }
                this.review.wanted = None;
                let remember = match show {
                    Show::Page { remember } => remember,
                    Show::InPlace if matches!(this.page, Page::Review { .. }) => false,
                    // The user left the page meanwhile.
                    Show::InPlace => return,
                };
                match result {
                    Ok(detail) => this.set_page(
                        Page::Review {
                            open: Some(id),
                            doc: Rc::new(live::proposal_page(&detail)),
                        },
                        remember,
                    ),
                    Err(error) => {
                        log::warn!("could not load the proposal {id}: {error}");
                        this.error = Some(error);
                    }
                }
            },
        );
    }

    /// A click on the row `id` of the queue.
    fn click_review_row(&mut self, id: &str, how: Click, cx: &mut Context<Self>) {
        let order = self.review_order();
        if let Some(open) = self.review.selection.click(&order, id, how)
            && self.review_open() != Some(open.as_str())
        {
            self.show_proposal(open, Show::InPlace, cx);
        }
        cx.notify();
    }

    /// What the buttons decide: the selection when it holds several, else
    /// the open proposal while it waits.
    fn review_targets(&self) -> Vec<String> {
        let order = self.review_order();
        if self.review.selection.len() > 1 {
            return self.review.selection.in_order(&order);
        }
        self.review_open()
            .filter(|open| order.iter().any(|id| id == open))
            .map(|open| vec![open.to_string()])
            .unwrap_or_default()
    }

    /// Approve or reject what the buttons decide.
    fn decide_review(&mut self, approve: bool, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        let ids = self.review_targets();
        if ids.is_empty() {
            return;
        }
        let (order, items) = (self.review_order(), self.pending_proposals.clone());
        let current = self.review_open().map(str::to_string);
        log::info!(
            "{} {} proposals of {}",
            if approve { "approving" } else { "rejecting" },
            ids.len(),
            repo.name
        );
        self.run(
            if approve { "Approving" } else { "Rejecting" },
            cx,
            move |b| decide(b, &repo.id, approve, &ids),
            move |this, results, cx| {
                for notice in review::decided_notices(approve, &results, &items) {
                    this.notify_user(notice, cx);
                }
                let decided: Vec<String> = results.into_iter().map(|result| result.id).collect();
                this.reload_notes(cx, move |this, cx| {
                    if !matches!(this.page, Page::Review { .. }) {
                        return;
                    }
                    let pending = this.review_order();
                    match review::after_decision(&order, &pending, &decided, current.as_deref()) {
                        Some(next) if current.as_deref() == Some(next.as_str()) => {}
                        Some(next) => this.show_proposal(next, Show::InPlace, cx),
                        None => this.set_page(empty_review(), false),
                    }
                });
            },
        );
    }

    /// Take the counts of waiting proposals from an answer of
    /// `GET /v1/status`; `None` when there is no answer that counts them. A
    /// count that rose is news: the user is told, and the open repository's
    /// queue loads again, quietly.
    pub(crate) fn take_proposal_counts(
        &mut self,
        counts: Option<HashMap<String, usize>>,
        cx: &mut Context<Self>,
    ) {
        let Some(counts) = counts else {
            self.review.counts = None;
            return;
        };
        let open = self.repo.as_ref().map(|repo| repo.id.clone());
        let own_work = review::own_work(self.busy.as_deref()) || self.review.asked_during_own_work;
        let news = review::news(
            self.review.counts.as_ref(),
            &counts,
            open.as_deref(),
            own_work,
        );
        self.review.counts = Some(counts);
        for (id, count, increase) in news {
            log::info!("{increase} new proposals in {id}; {count} wait");
            if open.as_deref() == Some(id.as_str()) && self.notes_loaded {
                self.reload_notes(cx, |this, cx| this.review_queue_grew(cx));
            }
            let name = self
                .repos
                .iter()
                .chain(&self.missing)
                .find(|repo| repo.id == id)
                .map_or_else(|| id.clone(), |repo| repo.name.clone());
            self.notify_user(review::rise_notice(&id, &name, increase), cx);
        }
    }

    /// The queue loaded again with new proposals in it. The review page that
    /// said nothing waits opens the first.
    fn review_queue_grew(&mut self, cx: &mut Context<Self>) {
        if matches!(self.page, Page::Review { open: None, .. }) {
            self.show_review(Show::InPlace, cx);
        }
    }

    /// The review page: the queue beside the open proposal, and above the
    /// proposal the buttons that decide. With nothing waiting, the page
    /// alone says so.
    pub(crate) fn review_page(&self, doc: &Rc<Doc>, cx: &mut Context<Self>) -> AnyElement {
        let detail = v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .children(self.review_actions(cx))
            .children(self.review_clash(cx))
            .child(self.doc_element(doc, cx));
        if self.pending_proposals.is_empty() {
            return detail.into_any_element();
        }
        h_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(self.review_queue(cx))
            .child(detail)
            .into_any_element()
    }

    /// The pending proposals, each with its checkbox, its note, its kind and
    /// why it was made; above them the checkbox that selects every one.
    fn review_queue(&self, cx: &mut Context<Self>) -> Div {
        let order = self.review_order();
        let selection = &self.review.selection;
        let open = self.review_open();
        let muted = cx.theme().muted_foreground;
        let label = match selection.len() {
            0 => format!("{} waiting", order.len()),
            selected => format!("{selected} of {} selected", order.len()),
        };
        let rows: Vec<ListItemButton> = self
            .pending_proposals
            .iter()
            .map(|item| {
                let (row, check) = (item.id.clone(), item.id.clone());
                let tone = match item.kind {
                    Kind::Sync => Tone::Info,
                    Kind::Agent => Tone::Tertiary,
                };
                let checkbox =
                    Checkbox::new(SharedString::from(format!("review-check:{}", item.id)))
                        .check(Check::of(selection.contains(&item.id)))
                        .edge_start()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.review.selection.toggle(&check);
                            cx.notify();
                        }));
                let row_button = ListItemButton::new(
                    SharedString::from(format!("review-row:{}", item.id)),
                    item.name().to_string(),
                )
                .selected(open == Some(item.id.as_str()))
                .leading(checkbox)
                .trailing(feedback::chip(item.kind.label(), tone, false, cx))
                .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                    let keys = event.modifiers();
                    this.click_review_row(&row, Click::with_keys(keys.control, keys.shift), cx);
                }));
                if item.reason.is_empty() {
                    row_button
                } else {
                    row_button.secondary(item.reason.clone())
                }
            })
            .collect();
        v_flex()
            .w(px(QUEUE_WIDTH))
            .h_full()
            .flex_shrink_0()
            .border_r_1()
            .border_color(glass::tinted_border(cx))
            .child(
                h_flex()
                    .h(px(QUEUE_HEAD))
                    .flex_shrink_0()
                    .px(px(list::ROW_START))
                    .gap_1()
                    .items_center()
                    .child(
                        Checkbox::new("review-all")
                            .check(selection.header(&order))
                            .edge_start()
                            .on_click(cx.listener(|this, _, _, cx| {
                                let order = this.review_order();
                                this.review.selection.toggle_all(&order);
                                cx.notify();
                            })),
                    )
                    .child(div().text_sm().text_color(muted).child(label)),
            )
            .child(
                div()
                    .id("review-queue")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(v_flex().pb_2().children(rows)),
            )
    }

    /// The buttons above the open proposal: for it alone while one or none
    /// is selected, for the selection while it holds several.
    fn review_actions(&self, cx: &mut Context<Self>) -> Option<Div> {
        let (approve, reject) = match self.review.selection.len() {
            several if several > 1 => (format!("Approve {several}"), format!("Reject {several}")),
            _ if !self.review_targets().is_empty() => {
                ("Approve and write".to_string(), "Reject".to_string())
            }
            _ => return None,
        };
        let busy = self.busy.is_some();
        Some(
            h_flex()
                .gap_2()
                .p_2()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    Button::new("approve")
                        .label(approve)
                        .small()
                        .contained()
                        .disabled(busy)
                        .on_click(cx.listener(|this, _, _, cx| this.decide_review(true, cx))),
                )
                .child(
                    Button::new("reject")
                        .label(reject)
                        .small()
                        .error()
                        .disabled(busy)
                        .on_click(cx.listener(|this, _, _, cx| this.decide_review(false, cx))),
                ),
        )
    }

    /// The warning while a sync's and an agent's proposal for one note are
    /// both selected.
    fn review_clash(&self, cx: &App) -> Option<Div> {
        if self.review.selection.len() < 2 {
            return None;
        }
        let warning = review::clash_warning(&self.pending_proposals, &self.review.selection)?;
        Some(
            div()
                .p_2()
                .child(feedback::alert(Tone::Warning, warning, cx)),
        )
    }

    /// The details pane beside the review page: how many proposals wait, of
    /// which kind, and how many are selected.
    pub(crate) fn review_details(&self, cx: &App) -> Div {
        let muted = cx.theme().muted_foreground;
        let lines = review::counts(&self.pending_proposals, self.review.selection.len());
        v_flex()
            .gap_3()
            .child(surfaces::text(Variant::Overline, "REVIEW").text_color(cx.theme().link))
            .child(
                v_flex().gap_1().children(
                    lines
                        .into_iter()
                        .map(|line| div().text_xs().text_color(muted).child(line)),
                ),
            )
    }
}

/// Decide the proposals `ids` of the repository `repo`. One goes by its own
/// route, which every backend has, and a refusal is its result; several go
/// in one call, which an older backend refuses with an error that says so.
/// Blocks; run it off the UI thread.
fn decide(
    backend: &Backend,
    repo: &str,
    approve: bool,
    ids: &[String],
) -> Result<Vec<DecideResult>, String> {
    let [id] = ids else {
        return backend.decide_proposals(repo, approve, ids);
    };
    let verb = if approve { "approve" } else { "reject" };
    let answer = backend.post(
        &format!("/v1/repos/{repo}/proposals/{id}/{verb}"),
        &json!({}),
    );
    Ok(vec![match answer {
        Ok(done) => DecideResult {
            id: id.clone(),
            ok: true,
            note_path: done["note_path"].as_str().map(str::to_string),
            error: None,
        },
        Err(error) => DecideResult {
            id: id.clone(),
            ok: false,
            note_path: None,
            error: Some(error),
        },
    }])
}
