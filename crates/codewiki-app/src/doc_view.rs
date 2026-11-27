//! Draws a [`Doc`] with the `mui` components.
//!
//! The page is a view of its own, [`DocPage`], drawn cached. A ripple or a
//! hover in the window's chrome then redraws the chrome and leaves the page
//! as it was; before this, an animation anywhere rebuilt the page on every
//! frame, which on a long table took 40% of a processor core.
//!
//! GPUI reuses a cached view only while no cached view around it is being
//! redrawn. The app is itself drawn cached, inside the backdrop's shell, so
//! a page drawn inside the app would be rebuilt with it. The shell therefore
//! draws the page, beside the app: the app leaves an empty place for it and
//! says where that is through a [`PageSlot`].
//!
//! A page is a column of blocks, as wide as a table needs and no wider. What
//! a link, a row or a button does is not decided here: every one reports its
//! [`Action`] to the caller.

use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

use gpui_kit::base::motion as kit;
use gpui_kit::component::input::{Escape, InputState, MoveDown, MoveUp};
use gpui_kit::component::scroll::Scrollbar;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnimationExt as _, AnyElement, App, Bounds, Context, Div, ElementId, Entity, EntityId,
    FontWeight, HighlightStyle, Hsla, InteractiveElement as _, InteractiveText, IntoElement,
    ListAlignment, ListOffset, ListState, ParentElement as _, Pixels, Render, RenderOnce,
    SharedString, StatefulInteractiveElement as _, Styled as _, StyledText, Window, div, img, list,
    relative,
};

use crate::doc::{Action, Block, Cell, Choice, Doc, Row, SecretState, Span, Stat, Table};
use crate::icons;
use crate::mui::base::Hover;
use crate::mui::button::{Button, ToggleButtonGroup};
use crate::mui::feedback::{self, Tone};
use crate::mui::list::ListItemButton;
use crate::mui::menu::AnchorBounds;
use crate::mui::motion::{self, Enter, EnterExt as _, GENTLE, STAGGER_CAP, scaled};
use crate::mui::select::Select;
use crate::mui::surfaces::{self, Piece, card, card_content, card_piece};
use crate::mui::switch::{self, Switch};
use crate::mui::text_field::{self, TextField};
use crate::mui::tokens::{Variant, curve, duration, easing};
use crate::reading;
use crate::settings::{FieldId, Note};
use crate::settings_layout as layout;
use crate::zoom::px;
use crate::{themes, zoom};

/// What a link, row or button of a page does when used.
pub type OnAction = Rc<dyn Fn(&Action, &mut Window, &mut App)>;

/// Gives the page's root the app's shortcut handlers. The shell draws the
/// page beside the app, so a focused input or button in it is not under the
/// app's root, which is where the shortcuts are handled.
pub type Shortcuts = Rc<dyn Fn(Div) -> Div>;
/// What a click on a select does: its owner opens the list.
pub type OnSelect = Rc<dyn Fn(SelectRequest, &mut Window, &mut App)>;

/// Space between two blocks, and between a heading and what it heads.
const BLOCK_GAP: f32 = 16.0;
const SECTION_GAP: f32 = 12.0;
/// MUI's `TableCell`, `size: 'small'`: `padding: '6px 16px'`.
const CELL_PADDING: (f32, f32) = (16.0, 6.0);
/// The bar of a share cell, and the room its label takes beside it.
const SHARE_BAR: f32 = 56.0;
const SHARE_GAP: f32 = 8.0;
const SHARE_LABEL: f32 = 24.0;
/// The narrowest a stat tile or a theme card gets before the row wraps.
pub(crate) const TILE_MIN: f32 = 180.0;
/// TradingAgent, `MuiTableRow`: the primary color at 5% under the pointer.
const ROW_HOVER: f32 = 0.05;
/// TradingAgent, `MuiTableCell`: the muted color at 30% between rows, and
/// the sidebar color at 40% behind the head.
const ROW_LINE: f32 = 0.3;
const HEAD_FILL: f32 = 0.4;
/// The logo on Home.
const LOGO_SIZE: f32 = 96.0;

/// What the blocks that hold live controls need from the view that owns
/// them: the text inputs, and the list of a select.
///
/// Only a page drawn with [`column`] has these. The page view, [`DocPage`],
/// is drawn by the shell from its cache, which suits a report that is read;
/// a text input and a menu belong to the view that handles their events.
pub struct Controls {
    /// The text input of each field.
    pub fields: HashMap<FieldId, Entity<InputState>>,
    /// The select whose list is open, by the path of its block.
    pub open_select: Option<String>,
    /// Called when a select is asked for its list.
    pub on_select: OnSelect,
}

/// The input of a page's [`Block::Search`], as Home's. The app owns it and
/// handles its events, and draws the list of what it finds under the field.
///
/// The cached page can hold a text input because the input is a view of
/// its own: what is typed redraws it and the views around it.
pub struct SearchSlot {
    pub input: Entity<InputState>,
    /// Where the field is drawn, for the list to hang under.
    pub anchor: AnchorBounds,
    /// The view that draws the list, told when the field moves.
    pub owner: EntityId,
    /// Called with the keys that act on the list. The shell draws the page
    /// beside the app, so the app's own key handlers never see a key typed
    /// in this field. True when the key was used, which keeps it from the
    /// input.
    pub on_key: OnSearchKey,
}

/// A key typed in a page's search field that acts on its list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKey {
    Escape,
    Up,
    Down,
}

/// What a key of a page's search field does; see [`SearchSlot::on_key`].
pub type OnSearchKey = Rc<dyn Fn(SearchKey, &mut Window, &mut App) -> bool>;

/// A select that was clicked: where it is, and what it offers.
pub struct SelectRequest {
    /// The path of the block, to tell this select from the others.
    pub path: String,
    pub anchor: Bounds<Pixels>,
    pub options: Vec<Choice>,
}

/// What every block needs to draw itself.
struct Page {
    /// Names the page among the others drawn since the app started, so that
    /// remembered state of one page does not leak into the next.
    serial: usize,
    on_action: OnAction,
    controls: Option<Rc<Controls>>,
    search: Option<Rc<SearchSlot>>,
}

impl Page {
    fn id(&self, path: &str) -> ElementId {
        ElementId::Name(format!("doc{}:{path}", self.serial).into())
    }
}

/// Where in the window the page goes. The app, which lays the window out,
/// writes it; the shell, which draws the page, reads it.
#[derive(Default)]
pub struct PageSlot {
    bounds: std::cell::Cell<Option<Bounds<Pixels>>>,
    /// The view to draw again when the place changes.
    reader: std::cell::Cell<Option<EntityId>>,
}

impl PageSlot {
    /// The place, in window coordinates; `None` while no generated page is
    /// shown.
    pub fn bounds(&self) -> Option<Bounds<Pixels>> {
        self.bounds.get()
    }

    /// Name the view that draws the page.
    pub fn read_by(&self, reader: EntityId) {
        self.reader.set(Some(reader));
    }

    /// Say where the page goes. Cheap to call on every frame: the reader is
    /// only told when the place has changed.
    pub fn set(&self, bounds: Option<Bounds<Pixels>>, cx: &mut App) {
        if self.bounds.get() == bounds {
            return;
        }
        self.bounds.set(bounds);
        if let Some(reader) = self.reader.get() {
            notify_after_draw(reader, cx);
        }
    }
}

/// Ask for `view` to be drawn again, from a place that runs while a frame is
/// being drawn.
///
/// GPUI wakes the window only for a notify made outside drawing. One made
/// during a frame marks the view and asks for no frame, so the change waited
/// for whatever asked next: the backdrop's drift, or any input. Under reduced
/// motion nothing drifts, and a new page stayed blank until the pointer
/// moved. Deferred, the notify runs once the frame is done, and wakes the
/// window.
pub fn notify_after_draw(view: EntityId, cx: &mut App) {
    cx.defer(move |cx| cx.notify(view));
}

/// A table with more rows than this is drawn a row at a time, so that only
/// the rows on screen are built.
const SPLIT_ROWS: usize = 12;
/// How far past the edges of the view the list builds, so that a scroll does
/// not show a gap.
const OVERDRAW: f32 = 400.0;

/// One strip of a page, as the list scrolls it: the title, a whole block, or
/// a piece of a long table.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Strip {
    Title,
    Block(usize),
    /// The head of the table that is block `.0`.
    TableHead(usize),
    /// Row `.1` of the table that is block `.0`.
    TableRow(usize, usize),
}

/// The strips of a page, top to bottom.
fn strips(doc: &Doc) -> Vec<Strip> {
    let mut out = vec![Strip::Title];
    for (index, block) in doc.blocks.iter().enumerate() {
        match block {
            Block::Table(table) if table.rows.len() > SPLIT_ROWS => {
                out.push(Strip::TableHead(index));
                out.extend((0..table.rows.len()).map(|row| Strip::TableRow(index, row)));
            }
            _ => out.push(Strip::Block(index)),
        }
    }
    out
}

/// A generated page as a view of its own, so that it can be drawn cached.
///
/// The page is a list of strips, and the list builds only the strips on
/// screen. A table of two hundred rows then costs what the thirty in view
/// cost, on every frame of a scroll, a hover or an entrance.
pub struct DocPage {
    doc: Rc<Doc>,
    /// Changes with every page shown.
    serial: usize,
    /// How wide the space for the page is.
    width: f32,
    on_action: OnAction,
    shortcuts: Shortcuts,
    strips: Rc<Vec<Strip>>,
    list: ListState,
    /// The zoom the strips were measured at. The list keeps the height of
    /// each strip it has measured, and a strip off screen is not measured
    /// again by itself.
    zoom_seen: f32,
    /// The input of the page's search field, while the page has one.
    search: Option<Rc<SearchSlot>>,
}

impl DocPage {
    // The room the list builds past its edges is set once, in the screen's
    // pixels.
    #[allow(clippy::disallowed_methods)]
    pub fn new(on_action: OnAction, shortcuts: Shortcuts) -> Self {
        let doc = Rc::new(Doc::default());
        let strips = Rc::new(strips(&doc));
        Self {
            list: ListState::new(strips.len(), ListAlignment::Top, gpui_kit::px(OVERDRAW)),
            doc,
            serial: 0,
            width: 0.0,
            on_action,
            shortcuts,
            strips,
            zoom_seen: zoom::factor(),
            search: None,
        }
    }

    /// Give the page's [`Block::Search`] its input, or take it away. Cheap
    /// to call on every frame, as [`Self::show`] is.
    pub fn set_search(&mut self, search: Option<Rc<SearchSlot>>, cx: &mut Context<Self>) {
        let same = match (&self.search, &search) {
            (Some(old), Some(new)) => Rc::ptr_eq(old, new),
            (None, None) => true,
            _ => false,
        };
        if !same {
            self.search = search;
            notify_after_draw(cx.entity_id(), cx);
        }
    }

    /// Show `doc` in a space `width` wide. Does nothing when that is what is
    /// shown already, so it can be called on every frame; the app calls it
    /// while it is drawn.
    pub fn show(&mut self, doc: &Rc<Doc>, serial: usize, width: f32, cx: &mut Context<Self>) {
        let same_doc = Rc::ptr_eq(&self.doc, doc) || self.doc == *doc;
        if same_doc && self.serial == serial && self.width == width {
            return;
        }
        if !same_doc || self.serial != serial {
            self.strips = Rc::new(strips(doc));
            // A new page starts at its top. The same page with new content,
            // a live update, stays where the reader was.
            let top = self.list.logical_scroll_top();
            self.list.reset(self.strips.len());
            if self.serial == serial {
                self.list.scroll_to(kept_top(top, self.strips.len()));
            }
        }
        self.doc = doc.clone();
        self.serial = serial;
        self.width = width;
        notify_after_draw(cx.entity_id(), cx);
    }
}

/// Where a page whose content changed in place goes back to, now that it
/// has `len` strips: where it was, or the top of its last strip when it was
/// further down than the page now reaches.
fn kept_top(old: ListOffset, len: usize) -> ListOffset {
    if old.item_ix < len {
        old
    } else {
        ListOffset {
            item_ix: len.saturating_sub(1),
            offset_in_item: Pixels::ZERO,
        }
    }
}

impl Render for DocPage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        if self.zoom_seen != zoom::factor() {
            self.zoom_seen = zoom::factor();
            self.list.remeasure();
        }
        let page = Rc::new(Page {
            serial: self.serial,
            on_action: self.on_action.clone(),
            controls: None,
            search: self.search.clone(),
        });
        let entrance = page.id("in");
        let (doc, strips) = (self.doc.clone(), self.strips.clone());
        let gutter = reading::gutter(self.width, reading::WIDE_MEASURE);
        let count = strips.len();
        let items = list(self.list.clone(), move |index, window, cx| {
            let last = index + 1 == count;
            draw_strip(&doc, strips[index], index, last, gutter, &page, window, cx)
        })
        .size_full();
        (self.shortcuts)(div())
            .size_full()
            .relative()
            .child(items)
            .child(Scrollbar::vertical(&self.list))
            // TradingAgent's `pageEntrance`: the page rises into place.
            .enter(entrance, Enter::PAGE)
    }
}

/// One strip of the page, with the page's gutters beside it.
#[allow(clippy::too_many_arguments)]
fn draw_strip(
    doc: &Doc,
    strip: Strip,
    index: usize,
    last: bool,
    gutter: f32,
    page: &Page,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let frame = div().px(px(gutter));
    let frame = match strip {
        Strip::Title => {
            let muted = cx.theme().muted_foreground;
            frame.pt(px(8.)).pb(px(BLOCK_GAP)).child(
                v_flex()
                    .gap(px(4.))
                    .child(surfaces::text(Variant::H3, doc.title.clone()))
                    .when_some(doc.lead.clone(), |head, lead| {
                        head.child(surfaces::text(Variant::Body1, lead).text_color(muted))
                    }),
            )
        }
        Strip::Block(block) => frame.pb(px(BLOCK_GAP)).child(draw(
            &doc.blocks[block],
            &block.to_string(),
            page,
            window,
            cx,
        )),
        Strip::TableHead(block) => match &doc.blocks[block] {
            Block::Table(table) => {
                frame.child(table_text(card_piece(Piece::Top, cx)).child(table_head(table, cx)))
            }
            _ => frame,
        },
        Strip::TableRow(block, row) => match &doc.blocks[block] {
            Block::Table(table) => {
                let end = row + 1 == table.rows.len();
                let piece = if end { Piece::Bottom } else { Piece::Middle };
                frame.when(end, |frame| frame.pb(px(BLOCK_GAP))).child(
                    table_text(card_piece(piece, cx)).child(table_row(
                        table,
                        row,
                        &block.to_string(),
                        page,
                        window,
                        cx,
                    )),
                )
            }
            _ => frame,
        },
    };
    let frame = frame.when(last, |frame| frame.pb(px(48.)));
    // TradingAgent's `staggerCascade`: the first strips of a page arrive one
    // after another. Table rows make their own, quicker entrance.
    match strip {
        Strip::Title | Strip::Block(_) | Strip::TableHead(_) if index < STAGGER_CAP => frame
            .enter(page.id(&format!("strip{index}:in")), Enter::card(index))
            .into_any_element(),
        _ => frame.into_any_element(),
    }
}

fn draw(block: &Block, path: &str, page: &Page, window: &mut Window, cx: &mut App) -> AnyElement {
    match block {
        Block::Heading(title) => surfaces::text(Variant::H4, title.clone())
            .mt(px(SECTION_GAP))
            .into_any_element(),
        Block::Text(spans) => line(spans, path, page, cx),
        Block::Stats(stats) => h_flex()
            .items_stretch()
            .flex_wrap()
            .gap(px(BLOCK_GAP))
            .children(
                stats
                    .iter()
                    .enumerate()
                    .map(|(index, stat)| stat_tile(stat, page.id(&format!("{path}.{index}")), cx)),
            )
            .into_any_element(),
        Block::Table(table) => draw_table(table, path, page, window, cx),
        Block::Card { title, chips, body } => {
            let mut content = card_content().child(
                h_flex()
                    .flex_wrap()
                    .gap(px(8.))
                    .items_center()
                    .child(surfaces::text(Variant::H4, title.clone()))
                    .children(
                        chips
                            .iter()
                            .map(|(label, tone)| feedback::chip(label.clone(), *tone, false, cx)),
                    ),
            );
            for (index, block) in body.iter().enumerate() {
                content = content.child(draw(block, &format!("{path}.{index}"), page, window, cx));
            }
            card(cx).child(content).into_any_element()
        }
        Block::List { items, ordered } => v_flex()
            .gap(px(4.))
            .children(items.iter().enumerate().map(|(index, item)| {
                let mark = if *ordered {
                    format!("{}.", index + 1)
                } else {
                    "•".to_string()
                };
                h_flex()
                    .items_start()
                    .gap(px(8.))
                    .child(
                        surfaces::text(Variant::Body2, mark)
                            .w(px(20.))
                            .flex_shrink_0()
                            .text_color(cx.theme().muted_foreground),
                    )
                    .child(div().flex_1().min_w_0().child(line(
                        item,
                        &format!("{path}.{index}"),
                        page,
                        cx,
                    )))
            }))
            .into_any_element(),
        Block::Rows(rows) => {
            card(cx)
                .child(v_flex().relative().py(px(8.)).children(
                    rows.iter().enumerate().map(|(index, row)| {
                        draw_row(row, page.id(&format!("{path}.{index}")), page)
                    }),
                ))
                .into_any_element()
        }
        Block::Code { language, text } => code_block(language, text, cx),
        Block::Alert { tone, text, button } => feedback::alert(*tone, text.clone(), cx)
            .when_some(button.clone(), |alert, (label, action)| {
                let on_action = page.on_action.clone();
                alert.child(feedback::alert_action(
                    Button::new(page.id(path))
                        .label(label)
                        .text()
                        .small()
                        .on_click(move |_, window, cx| on_action(&action, window, cx)),
                ))
            })
            .into_any_element(),
        Block::Empty(text) => empty(text, cx),
        Block::Setting {
            label,
            value,
            button,
        } => {
            // A value that is shown, not set here, with one thing to do
            // about it: as tall as a control, so that the rows line up.
            let control = h_flex()
                .min_h(px(text_field::HEIGHT))
                .gap(px(12.))
                .items_center()
                .child(
                    surfaces::text(Variant::Body2, value.clone())
                        .flex_1()
                        .min_w_0(),
                )
                .when_some(button.clone(), |row, (label, action)| {
                    let on_action = page.on_action.clone();
                    row.child(
                        Button::new(page.id(path))
                            .label(label)
                            .small()
                            .on_click(move |_, window, cx| on_action(&action, window, cx)),
                    )
                });
            row(label, &None, control, None, true, cx)
        }
        Block::Choices { label, choices } => choices_row(label, choices, path, page, cx),
        Block::Switch {
            label,
            help,
            on,
            action,
            locked,
        } => {
            let (on_action, action) = (page.on_action.clone(), action.clone());
            let switch = Switch::new(SharedString::from(format!("doc{}:{path}", page.serial)))
                .on(*on)
                .disabled(locked.is_some())
                .on_change(move |_, window, cx| on_action(&action, window, cx));
            // The track lines up with the controls above and below it.
            let control = div().ml(px(-switch::INSET)).child(switch);
            setting_row(label, help, control, under(locked, &None), cx)
        }
        Block::Select {
            label,
            help,
            options,
            locked,
            note,
        } => {
            let control = select_control(options, locked.is_some(), path, page, window, cx);
            setting_row(label, help, control, under(locked, note), cx)
        }
        Block::Field {
            id,
            label,
            help,
            locked,
            note,
        } => {
            let control = match page.controls.as_ref().and_then(|c| c.fields.get(id)) {
                Some(state) => TextField::new(page.id(path), state)
                    .disabled(locked.is_some())
                    .when_some(under(locked, note), |field, (text, tone)| {
                        field.helper(text, tone)
                    })
                    .into_any_element(),
                None => div().into_any_element(),
            };
            setting_row(label, help, control, None, cx)
        }
        Block::Secret {
            id,
            label,
            help,
            state,
            locked,
            note,
        } => {
            let control = secret_control(*id, state, locked, note, path, page, cx);
            setting_row(label, help, control, None, cx)
        }
        Block::Buttons(buttons) => h_flex()
            .flex_wrap()
            .gap(px(8.))
            .children(buttons.iter().enumerate().map(|(index, press)| {
                let (on_action, action) = (page.on_action.clone(), press.action.clone());
                Button::new(page.id(&format!("{path}.{index}")))
                    .label(press.label.clone())
                    .when(press.primary, Button::contained)
                    .disabled(press.disabled)
                    .on_click(move |_, window, cx| on_action(&action, window, cx))
            }))
            .into_any_element(),
        Block::Progress {
            label,
            share,
            detail,
            moving,
        } => {
            let theme = cx.theme();
            let (color, muted) = (theme.primary, theme.muted_foreground);
            // Work that waits keeps its bar, still and without its color.
            let bar = match share {
                Some(share) => feedback::progress(*share, if *moving { color } else { muted }, cx)
                    .w_full()
                    .into_any_element(),
                None if *moving => feedback::linear_progress("model-job", cx).into_any_element(),
                None => feedback::progress(0.0, muted, cx)
                    .w_full()
                    .into_any_element(),
            };
            v_flex()
                .gap(px(8.))
                .child(surfaces::text(Variant::Body2, label.clone()))
                .child(bar)
                .child(surfaces::text(Variant::Caption, detail.clone()).text_color(muted))
                .into_any_element()
        }
        Block::Logo => img(icons::LOGO)
            .size(px(LOGO_SIZE))
            .flex_none()
            .into_any_element(),
        Block::Search => match &page.search {
            Some(search) => {
                let owner = search.owner;
                let key = |key: SearchKey| {
                    let on_key = search.on_key.clone();
                    move |window: &mut Window, cx: &mut App| {
                        if on_key(key, window, cx) {
                            cx.stop_propagation();
                        }
                    }
                };
                let (escape, up, down) = (
                    key(SearchKey::Escape),
                    key(SearchKey::Up),
                    key(SearchKey::Down),
                );
                search
                    .anchor
                    .track(
                        TextField::new(page.id(path), &search.input).icon(icons::SEARCH),
                        // The open list follows the field as the page
                        // scrolls or the window is resized.
                        move |_, cx| notify_after_draw(owner, cx),
                    )
                    .w_full()
                    // Before the input, which would use them itself.
                    .capture_action(move |_: &Escape, window, cx| escape(window, cx))
                    .capture_action(move |_: &MoveUp, window, cx| up(window, cx))
                    .capture_action(move |_: &MoveDown, window, cx| down(window, cx))
                    .into_any_element()
            }
            None => div().into_any_element(),
        },
    }
}

/// A short page drawn whole, with live controls: a category of the
/// settings. The caller scrolls it.
pub fn column(
    doc: &Doc,
    serial: usize,
    on_action: OnAction,
    controls: Rc<Controls>,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    let page = Page {
        serial,
        on_action,
        controls: Some(controls),
        search: None,
    };
    let muted = cx.theme().muted_foreground;
    let head = v_flex()
        .gap(px(4.))
        .child(surfaces::text(Variant::H3, doc.title.clone()))
        .when_some(doc.lead.clone(), |head, lead| {
            head.child(surfaces::text(Variant::Body1, lead).text_color(muted))
        });
    let mut column = v_flex()
        .gap(px(BLOCK_GAP))
        .child(head.enter(page.id("title:in"), Enter::card(0)));
    for (index, block) in doc.blocks.iter().enumerate() {
        let drawn = draw(block, &index.to_string(), &page, window, cx);
        // The first blocks arrive one after another, as on every page.
        column = column.child(if index + 1 < STAGGER_CAP {
            div()
                .child(drawn)
                .enter(page.id(&format!("strip{index}:in")), Enter::card(index + 1))
                .into_any_element()
        } else {
            drawn
        });
    }
    column
}

/// What stands under a control: why it cannot be changed, or else the news
/// about its last change.
fn under(locked: &Option<String>, note: &Option<Note>) -> Option<(String, Tone)> {
    if let Some(locked) = locked {
        return Some((locked.clone(), Tone::Neutral));
    }
    note.as_ref().map(|note| match note {
        Note::Working => ("Saving…".to_string(), Tone::Neutral),
        Note::Saved => ("Saved".to_string(), Tone::Success),
        Note::Error(error) => (error.clone(), Tone::Error),
    })
}

/// One line of small text under a control, in a tone's color.
fn caption(text: String, tone: Tone, cx: &App) -> Div {
    surfaces::text(Variant::Caption, text)
        .mt(px(4.))
        .text_color(tone.color(cx))
}

/// The row of one setting: its name, with a line of help under it, and its
/// control beside them.
fn setting_row(
    label: &str,
    help: &Option<String>,
    control: impl IntoElement,
    under: Option<(String, Tone)>,
    cx: &App,
) -> AnyElement {
    row(label, help, control, under, false, cx)
}

/// [`setting_row`], with the choice of how wide the control's side is:
/// `fill` gives it all the room that is left, for a value that is shown and
/// may be long, such as a folder.
fn row(
    label: &str,
    help: &Option<String>,
    control: impl IntoElement,
    under: Option<(String, Tone)>,
    fill: bool,
    cx: &App,
) -> AnyElement {
    let muted = cx.theme().muted_foreground;
    // A control is 40 px high and the name one line of 22: this puts the
    // name on the middle line of the control.
    let name_top = (text_field::HEIGHT - 14.0 * 1.57) / 2.0;
    h_flex()
        .items_start()
        .gap(px(layout::ROW_GAP))
        .py(px(layout::ROW_SPACING / 2.0))
        .child(
            v_flex()
                .w(px(layout::LABEL_WIDTH))
                .flex_shrink_0()
                .pt(px(name_top))
                .gap(px(2.))
                .child(
                    surfaces::text(Variant::Body2, label.to_string())
                        .font_weight(FontWeight::MEDIUM),
                )
                .when_some(help.clone(), |name, help| {
                    name.child(surfaces::text(Variant::Caption, help).text_color(muted))
                }),
        )
        .child(
            v_flex()
                .map(|side| {
                    if fill {
                        side.flex_1()
                    } else {
                        side.w(px(layout::CONTROL_WIDTH))
                    }
                })
                .min_w_0()
                .child(control)
                .when_some(under, |column, (text, tone)| {
                    column.child(caption(text, tone, cx))
                }),
        )
        .into_any_element()
}

/// The closed control of a select. A click asks the owner for its list.
fn select_control(
    options: &[Choice],
    disabled: bool,
    path: &str,
    page: &Page,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let chosen = options
        .iter()
        .find(|option| option.selected)
        .map(|option| option.label.clone())
        .unwrap_or_default();
    let Some(controls) = page.controls.clone() else {
        return surfaces::text(Variant::Body2, chosen).into_any_element();
    };
    // Where the control is, for its list to hang under.
    let anchor = window
        .use_keyed_state(page.id(&format!("{path}:anchor")), cx, |_, _| {
            AnchorBounds::default()
        })
        .read(cx)
        .clone();
    let open = controls.open_select.as_deref() == Some(path);
    let request = {
        let (anchor, path, options) = (anchor.clone(), path.to_string(), options.to_vec());
        move |_: &gpui_kit::ClickEvent, window: &mut Window, cx: &mut App| {
            if let Some(bounds) = anchor.get() {
                (controls.on_select)(
                    SelectRequest {
                        path: path.clone(),
                        anchor: bounds,
                        options: options.clone(),
                    },
                    window,
                    cx,
                );
            }
        }
    };
    // The icon of the choice in effect, as its row in the list shows it.
    let icon = options
        .iter()
        .find(|option| option.selected)
        .and_then(|option| Some((option.icon?, choice_tint(option, cx))));
    let select = Select::new(page.id(path), chosen)
        .open(open)
        .disabled(disabled)
        .when_some(icon, |select, (icon, tint)| select.icon(icon, tint))
        .on_click(request);
    anchor.track(select, |_, _| {}).w_full().into_any_element()
}

/// The color of a choice's icon. A theme's icon is in a color of that theme,
/// as in the theme menu; any other takes the hue its kind has everywhere.
pub fn choice_tint(choice: &Choice, cx: &App) -> Option<Hsla> {
    if let Action::Theme(name) = &choice.action {
        return themes::entries(cx)
            .iter()
            .find(|entry| entry.name == *name)
            .map(|entry| themes::glyph_tint(entry.swatches[1], cx));
    }
    choice.icon.and_then(|icon| icons::tint(icon, cx))
}

/// A setting that is typed once and hidden: its input, or what is known of
/// the saved one with the means to replace or forget it.
fn secret_control(
    id: FieldId,
    state: &SecretState,
    locked: &Option<String>,
    note: &Option<Note>,
    path: &str,
    page: &Page,
    cx: &App,
) -> AnyElement {
    let muted = cx.theme().muted_foreground;
    // The control's height, so that the row does not jump between states.
    let line = || {
        h_flex()
            .min_h(px(text_field::HEIGHT))
            .gap(px(8.))
            .items_center()
    };
    if let Some(locked) = locked {
        return line()
            .child(surfaces::text(Variant::Body2, locked.clone()).text_color(muted))
            .into_any_element();
    }
    let press = |suffix: &str, label: &str, action: Action| {
        let on_action = page.on_action.clone();
        Button::new(page.id(&format!("{path}:{suffix}")))
            .label(label.to_string())
            .small()
            .on_click(move |_, window, cx| on_action(&action, window, cx))
    };
    if let Some(text) = state.text() {
        return v_flex()
            .child(
                line()
                    .child(
                        surfaces::text(Variant::Body2, text)
                            .flex_1()
                            .min_w_0()
                            .font_weight(FontWeight::MEDIUM),
                    )
                    .child(press("replace", "Replace", Action::ReplaceSecret(id)))
                    .child(
                        press("remove", "Remove", Action::RemoveSecret(id))
                            .text()
                            .error(),
                    ),
            )
            .when_some(under(&None, note), |column, (text, tone)| {
                column.child(caption(text, tone, cx))
            })
            .into_any_element();
    }
    let Some(input) = page.controls.as_ref().and_then(|c| c.fields.get(&id)) else {
        return div().into_any_element();
    };
    let field = TextField::new(page.id(path), input)
        .when_some(under(&None, note), |field, (text, tone)| {
            field.helper(text, tone)
        });
    h_flex()
        .items_start()
        .gap(px(8.))
        .child(div().flex_1().min_w_0().child(field))
        .when(*state == SecretState::Typing, |row| {
            row.child(
                h_flex()
                    .h(px(text_field::HEIGHT))
                    .items_center()
                    .child(press("keep", "Cancel", Action::KeepSecret(id)).text()),
            )
        })
        .into_any_element()
}

/// A line of spans as one run of text that wraps, with its links live.
fn line(spans: &[Span], path: &str, page: &Page, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let mut text = String::new();
    let mut highlights: Vec<(Range<usize>, HighlightStyle)> = Vec::new();
    let mut code: Vec<(Range<usize>, SharedString)> = Vec::new();
    let mut links: Vec<(Range<usize>, Action, usize)> = Vec::new();
    for span in spans {
        let start = text.len();
        match span {
            Span::Text(part) => text.push_str(part),
            Span::Strong(part) => {
                text.push_str(part);
                highlights.push((
                    start..text.len(),
                    HighlightStyle {
                        font_weight: Some(FontWeight::SEMIBOLD),
                        ..Default::default()
                    },
                ));
            }
            Span::Code(part) => {
                text.push_str(part);
                code.push((start..text.len(), theme.mono_font_family.clone()));
                highlights.push((
                    start..text.len(),
                    HighlightStyle {
                        background_color: Some(theme.muted),
                        ..Default::default()
                    },
                ));
            }
            Span::Link {
                label,
                code: mono,
                action,
            } => {
                text.push_str(label);
                if *mono {
                    code.push((start..text.len(), theme.mono_font_family.clone()));
                }
                // The link's own highlight is the one its hover look joins.
                links.push((start..text.len(), action.clone(), highlights.len()));
                highlights.push((
                    start..text.len(),
                    HighlightStyle {
                        color: Some(theme.link),
                        ..Default::default()
                    },
                ));
            }
        }
    }
    if links.is_empty() {
        let spec = Variant::Body2.spec();
        let styled = StyledText::new(text)
            .with_highlights(highlights)
            .with_font_family_overrides(code);
        return div()
            .text_size(px(spec.size))
            .line_height(px(spec.size * spec.line_height))
            .child(styled)
            .into_any_element();
    }
    LinkedLine {
        id: page.id(&format!("{path}:text")),
        text: text.into(),
        highlights,
        code,
        links,
        on_action: page.on_action.clone(),
    }
    .into_any_element()
}

/// A line with links in it. The link under the pointer takes the hover look
/// of `reading::link_hover`, which needs the window to remember which one
/// it is.
#[derive(IntoElement)]
struct LinkedLine {
    id: ElementId,
    text: SharedString,
    highlights: Vec<(Range<usize>, HighlightStyle)>,
    code: Vec<(Range<usize>, SharedString)>,
    /// Each link's range, what it does, and which of `highlights` is its own.
    links: Vec<(Range<usize>, Action, usize)>,
    on_action: OnAction,
}

impl RenderOnce for LinkedLine {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let hovered = window.use_keyed_state(
            ElementId::NamedChild(self.id.clone().into(), "hovered-link".into()),
            cx,
            |_, _| None::<usize>,
        );
        let mut highlights = self.highlights;
        let look = reading::link_hover(cx);
        if let Some((_, _, own)) = hovered.read(cx).and_then(|link| self.links.get(link))
            && let Some((_, style)) = highlights.get_mut(*own)
        {
            *style = style.highlight(look);
        }
        let ranges: Vec<Range<usize>> = self
            .links
            .iter()
            .map(|(range, _, _)| range.clone())
            .collect();
        let actions: Vec<Action> = self
            .links
            .into_iter()
            .map(|(_, action, _)| action)
            .collect();
        let styled = StyledText::new(self.text)
            .with_highlights(highlights)
            .with_font_family_overrides(self.code);
        let on_action = self.on_action;
        let spec = Variant::Body2.spec();
        let (over, out) = (hovered.clone(), hovered);
        div()
            .id(self.id.clone())
            .text_size(px(spec.size))
            .line_height(px(spec.size * spec.line_height))
            // The text reports only a pointer over it; leaving the line
            // ends the hover look.
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    out.update(cx, |link, cx| {
                        if link.take().is_some() {
                            cx.notify();
                        }
                    });
                }
            })
            .child(
                InteractiveText::new(self.id, styled)
                    .on_hover({
                        let ranges = ranges.clone();
                        move |index, _, _, cx| {
                            let link = index.and_then(|index| {
                                ranges.iter().position(|range| range.contains(&index))
                            });
                            over.update(cx, |hovered, _| *hovered = link);
                        }
                    })
                    .on_click(ranges, move |index, window, cx| {
                        if let Some(action) = actions.get(index) {
                            on_action(action, window, cx);
                        }
                    }),
            )
    }
}

/// TradingAgent's metric tile: a label, a large figure that counts up when
/// the tile appears, and a note under it. `id` names the tile.
pub fn stat_tile(stat: &Stat, id: ElementId, cx: &App) -> Div {
    let theme = cx.theme();
    let figure = if stat.tone == Tone::Neutral {
        theme.foreground
    } else {
        stat.tone.color(cx)
    };
    card(cx).flex_1().min_w(px(TILE_MIN)).child(
        card_content()
            .gap(px(2.))
            .child(
                surfaces::text(Variant::Overline, stat.label.clone())
                    .line_height(px(20.))
                    .text_color(theme.muted_foreground),
            )
            .child({
                let value = stat.value.clone();
                let spec = Variant::H2.spec();
                div()
                    .text_size(px(spec.size))
                    .line_height(px(spec.size * spec.line_height))
                    .font_weight(spec.weight)
                    .text_color(figure)
                    .with_animation(id, GENTLE.animation(), move |figure, progress| {
                        figure.child(motion::count_up(&value, progress))
                    })
            })
            .when_some(stat.hint.clone(), |tile, hint| {
                tile.child(
                    surfaces::text(Variant::Caption, hint).text_color(theme.muted_foreground),
                )
            }),
    )
}

/// The text style of a table, on whatever holds its rows.
fn table_text(holder: Div) -> Div {
    let spec = Variant::Body2.spec();
    holder
        .text_size(px(spec.size))
        .line_height(px(spec.size * 1.43))
}

/// A cell of column `index`, taking that column's share of the width.
fn table_cell(table: &Table, index: usize) -> Div {
    let total: f32 = table.columns.iter().map(|column| column.weight).sum();
    let column = &table.columns[index];
    div()
        .flex_basis(relative(column.weight / total.max(f32::EPSILON)))
        .flex_grow(1.)
        .flex_shrink(1.)
        .min_w_0()
        // A column of share cells keeps room for its bar and label: in a
        // narrow window its share of the width was less, the label was
        // drawn over the next column and the head wrapped ("Scor" / "e").
        .when(holds_shares(table, index), |cell| {
            cell.min_w(px(SHARE_BAR
                + SHARE_GAP
                + SHARE_LABEL
                + 2. * CELL_PADDING.0))
        })
        .px(px(CELL_PADDING.0))
        .py(px(CELL_PADDING.1))
        .when(column.numeric, |cell| cell.flex().justify_end())
}

/// Whether column `index` of the table holds share cells.
fn holds_shares(table: &Table, index: usize) -> bool {
    table
        .rows
        .iter()
        .any(|row| matches!(row.get(index), Some(Cell::Share { .. })))
}

fn table_head(table: &Table, cx: &App) -> Div {
    let theme = cx.theme();
    h_flex()
        .bg(theme.sidebar.opacity(HEAD_FILL))
        .border_b_1()
        .border_color(theme.muted_foreground.opacity(ROW_LINE))
        .font_weight(FontWeight::SEMIBOLD)
        .children(
            table
                .columns
                .iter()
                .enumerate()
                .map(|(index, column)| table_cell(table, index).child(column.label.clone())),
        )
}

fn table_row(
    table: &Table,
    row_index: usize,
    path: &str,
    page: &Page,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let theme = cx.theme();
    let (primary, rule) = (theme.primary, theme.muted_foreground.opacity(ROW_LINE));
    let last = row_index + 1 == table.rows.len();
    let id = page.id(&format!("{path}.{row_index}"));
    let hover = Hover::new(&id, window, cx);
    let policy = kit::Transition::new(scaled(duration::SHORTER)).ease(easing(curve::EASE_IN_OUT));
    let target = if hover.hovered { 1.0 } else { 0.0 };
    let lit = kit::transition((id.clone(), "lit"), target, policy, window, cx);
    let cells = table.rows[row_index]
        .iter()
        .enumerate()
        .map(|(index, content)| {
            let path = format!("{path}.{row_index}.{index}");
            table_cell(table, index).child(draw_cell(content, &path, page, cx))
        });
    let element = h_flex()
        .id(id)
        .relative()
        .items_center()
        .bg(primary.opacity(ROW_HOVER * lit))
        .when(!last, |row| row.border_b_1().border_color(rule))
        // TradingAgent's bar down the left edge of the row under the
        // pointer. It scales in there; here it fades in.
        .child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .h_full()
                .w(px(3.))
                .bg(primary.opacity(lit)),
        )
        .children(cells);
    let element = hover.attach(element);
    // TradingAgent's `listStagger`, for the rows a reader sees first.
    if row_index < STAGGER_CAP {
        element
            .enter(
                page.id(&format!("{path}.{row_index}:in")),
                Enter::row(row_index),
            )
            .into_any_element()
    } else {
        element.into_any_element()
    }
}

/// A whole table in one card: for a short table, or one inside a card.
fn draw_table(
    table: &Table,
    path: &str,
    page: &Page,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let mut body = table_text(v_flex().relative()).child(table_head(table, cx));
    for row_index in 0..table.rows.len() {
        body = body.child(table_row(table, row_index, path, page, window, cx));
    }
    card(cx).child(body).into_any_element()
}

fn draw_cell(cell: &Cell, path: &str, page: &Page, cx: &App) -> AnyElement {
    match cell {
        Cell::Line(spans) => line(spans, path, page, cx),
        // In a row of its own, so that the chip is as wide as its label and
        // not as wide as its column.
        Cell::Chip { label, tone } => h_flex()
            .child(feedback::chip(label.clone(), *tone, false, cx))
            .into_any_element(),
        Cell::Share { label, share, tone } => {
            let color = if *tone == Tone::Neutral {
                cx.theme().primary
            } else {
                tone.color(cx)
            };
            h_flex()
                .gap(px(SHARE_GAP))
                .items_center()
                .child(feedback::progress(*share, color, cx).w(px(SHARE_BAR)))
                .child(label.clone())
                .into_any_element()
        }
    }
}

fn draw_row(row: &Row, id: ElementId, page: &Page) -> ListItemButton {
    let on_action = page.on_action.clone();
    let action = row.action.clone();
    let aside_id = id.clone();
    ListItemButton::new(id, row.label.clone())
        .icon(row.icon)
        .code(row.code)
        .selected(row.selected)
        .when_some(row.detail.clone(), |item, detail| item.secondary(detail))
        .when_some(row.aside.clone(), |item, (label, action)| {
            // The button takes its press alone, so the row does not open.
            let on_action = page.on_action.clone();
            item.trailing(
                Button::new((aside_id, "aside"))
                    .label(label)
                    .text()
                    .small()
                    .on_click(move |_, window, cx| on_action(&action, window, cx)),
            )
        })
        .on_click(move |_, window, cx| on_action(&action, window, cx))
}

/// Text in the code font, line for line. A diff gets its added and removed
/// lines colored.
fn code_block(language: &str, text: &str, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let diff = language == "diff";
    let color = |line: &str| -> Hsla {
        match line.as_bytes().first() {
            Some(b'+') if diff && !line.starts_with("+++") => theme.success,
            Some(b'-') if diff && !line.starts_with("---") => theme.danger,
            Some(b'@') if diff => theme.info,
            _ => theme.foreground,
        }
    };
    card(cx)
        .child(
            v_flex()
                .id(SharedString::from(format!("code:{}", text.len())))
                .relative()
                .p(px(surfaces::CARD_PADDING))
                .overflow_x_scroll()
                .font_family(theme.mono_font_family.clone())
                .text_size(px(13.))
                .line_height(px(20.))
                .children(text.lines().map(|line| {
                    // An empty line still takes a line's height.
                    let shown = if line.is_empty() { " " } else { line };
                    div()
                        .whitespace_nowrap()
                        .text_color(color(line))
                        .child(shown.to_string())
                })),
        )
        .into_any_element()
}

/// What a page shows when it has nothing to list.
fn empty(text: &str, cx: &App) -> AnyElement {
    let muted = cx.theme().muted_foreground;
    card(cx)
        .child(
            card_content()
                .items_center()
                .py(px(32.))
                .gap(px(12.))
                .child(
                    Icon::new(icons::INFO)
                        .with_size(px(32.))
                        .text_color(muted.opacity(0.7)),
                )
                .child(surfaces::text(Variant::Body1, text.to_string()).text_color(muted)),
        )
        .into_any_element()
}

fn choices_row(label: &str, choices: &[Choice], path: &str, page: &Page, cx: &App) -> AnyElement {
    let selected = choices.iter().position(|choice| choice.selected);
    let labels: Vec<SharedString> = choices
        .iter()
        .map(|choice| choice.label.clone().into())
        .collect();
    let actions: Vec<Action> = choices.iter().map(|choice| choice.action.clone()).collect();
    let on_action = page.on_action.clone();
    // As tall as the other controls, so that the rows of a card line up.
    let control = h_flex().min_h(px(text_field::HEIGHT)).items_center().child(
        ToggleButtonGroup::new(format!("doc{}:{path}", page.serial), labels)
            .selected(selected)
            .on_select(move |index, window, cx| {
                if let Some(action) = actions.get(index) {
                    on_action(action, window, cx);
                }
            }),
    );
    setting_row(label, &None, control, None, cx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::{Cell, Column};

    fn table(rows: usize) -> Block {
        let mut table = Table::new([Column::new("File")]);
        for row in 0..rows {
            table.row([Cell::text(row)]);
        }
        Block::Table(table)
    }

    #[test]
    fn a_short_table_is_one_strip_and_a_long_one_is_a_strip_a_row() {
        let mut doc = Doc::new("Report");
        doc.heading("Short");
        doc.push(table(SPLIT_ROWS));
        doc.push(table(SPLIT_ROWS + 1));
        let strips = strips(&doc);
        assert_eq!(
            &strips[..4],
            [
                Strip::Title,
                Strip::Block(0),
                Strip::Block(1),
                Strip::TableHead(2)
            ]
        );
        assert_eq!(strips.len(), 4 + SPLIT_ROWS + 1);
        assert_eq!(strips[4], Strip::TableRow(2, 0));
        assert_eq!(strips.last(), Some(&Strip::TableRow(2, SPLIT_ROWS)));
    }

    #[test]
    fn an_empty_page_is_only_its_title() {
        assert_eq!(strips(&Doc::default()), [Strip::Title]);
    }

    #[test]
    fn a_table_inside_a_card_stays_whole() {
        let mut doc = Doc::new("Settings");
        doc.push(Block::Card {
            title: "Keys".into(),
            chips: Vec::new(),
            body: vec![table(SPLIT_ROWS * 3)],
        });
        assert_eq!(strips(&doc), [Strip::Title, Strip::Block(0)]);
    }

    #[test]
    fn a_live_update_keeps_the_place_the_page_still_has() {
        let at = |item_ix: usize, offset: f32| ListOffset {
            item_ix,
            offset_in_item: Pixels::from(offset),
        };
        // A row added or removed below: the reader stays where they were.
        assert_eq!(kept_top(at(5, 12.0), 30), at(5, 12.0));
        assert_eq!(kept_top(at(0, 0.0), 1), at(0, 0.0));
        // The page got shorter than where the reader was: its last strip.
        assert_eq!(kept_top(at(20, 12.0), 8), at(7, 0.0));
        assert_eq!(kept_top(at(8, 3.0), 8), at(7, 0.0));
        // A page is never empty, yet nothing goes below the first strip.
        assert_eq!(kept_top(at(3, 1.0), 0), at(0, 0.0));
    }
}
