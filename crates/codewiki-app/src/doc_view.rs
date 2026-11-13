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

use std::ops::Range;
use std::rc::Rc;

use gpui_kit::base::motion as kit;
use gpui_kit::component::scroll::Scrollbar;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnimationExt as _, AnyElement, App, Bounds, Context, Div, ElementId, EntityId, FontWeight,
    HighlightStyle, Hsla, InteractiveElement as _, InteractiveText, IntoElement, ListAlignment,
    ListState, ParentElement as _, Pixels, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, StyledText, Window, div, list, px, relative,
};

use crate::doc::{Action, Block, Cell, Choice, Doc, Row, Span, Stat, Swatch, Table};
use crate::icons;
use crate::mui::base::Hover;
use crate::mui::button::{Button, CardButton, ToggleButtonGroup};
use crate::mui::feedback::{self, Tone};
use crate::mui::list::ListItemButton;
use crate::mui::motion::{self, Enter, EnterExt as _, GENTLE, STAGGER_CAP, scaled};
use crate::mui::surfaces::{self, Piece, card, card_content, card_piece};
use crate::mui::tokens::{Variant, curve, duration, easing};
use crate::reading;

/// What a link, row or button of a page does when used.
pub type OnAction = Rc<dyn Fn(&Action, &mut Window, &mut App)>;

/// Space between two blocks, and between a heading and what it heads.
const BLOCK_GAP: f32 = 16.0;
const SECTION_GAP: f32 = 12.0;
/// MUI's `TableCell`, `size: 'small'`: `padding: '6px 16px'`.
const CELL_PADDING: (f32, f32) = (16.0, 6.0);
/// The narrowest a stat tile or a theme card gets before the row wraps.
const TILE_MIN: f32 = 180.0;
/// TradingAgent, `MuiTableRow`: the primary color at 5% under the pointer.
const ROW_HOVER: f32 = 0.05;
/// TradingAgent, `MuiTableCell`: the muted color at 30% between rows, and
/// the sidebar color at 40% behind the head.
const ROW_LINE: f32 = 0.3;
const HEAD_FILL: f32 = 0.4;

/// What every block needs to draw itself.
struct Page {
    /// Names the page among the others drawn since the app started, so that
    /// remembered state of one page does not leak into the next.
    serial: usize,
    on_action: OnAction,
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
    strips: Rc<Vec<Strip>>,
    list: ListState,
}

impl DocPage {
    pub fn new(on_action: OnAction) -> Self {
        let doc = Rc::new(Doc::default());
        let strips = Rc::new(strips(&doc));
        Self {
            list: ListState::new(strips.len(), ListAlignment::Top, px(OVERDRAW)),
            doc,
            serial: 0,
            width: 0.0,
            on_action,
            strips,
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
            // A new page starts at its top.
            self.list.reset(self.strips.len());
        }
        self.doc = doc.clone();
        self.serial = serial;
        self.width = width;
        notify_after_draw(cx.entity_id(), cx);
    }
}

impl Render for DocPage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let page = Rc::new(Page {
            serial: self.serial,
            on_action: self.on_action.clone(),
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
        div()
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
        Block::Alert { tone, text } => feedback::alert(*tone, text.clone(), cx).into_any_element(),
        Block::Empty(text) => empty(text, cx),
        Block::Setting {
            label,
            value,
            button,
        } => h_flex()
            .gap(px(12.))
            .items_center()
            .child(
                surfaces::text(Variant::Body2, label.clone())
                    .w(px(120.))
                    .flex_shrink_0()
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                surfaces::text(Variant::Body2, value.clone())
                    .flex_1()
                    .min_w_0()
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .when_some(button.clone(), |row, (label, action)| {
                let on_action = page.on_action.clone();
                row.child(
                    Button::new(page.id(path))
                        .label(label)
                        .small()
                        .on_click(move |_, window, cx| on_action(&action, window, cx)),
                )
            })
            .into_any_element(),
        Block::Choices { label, choices } => choices_row(label, choices, path, page, cx),
        Block::Swatches(swatches) => h_flex()
            .flex_wrap()
            .gap(px(12.))
            .children(swatches.iter().enumerate().map(|(index, swatch)| {
                swatch_card(swatch, page.id(&format!("{path}.{index}")), page, cx)
            }))
            .into_any_element(),
    }
}

/// A line of spans as one run of text that wraps, with its links live.
fn line(spans: &[Span], path: &str, page: &Page, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let mut text = String::new();
    let mut highlights: Vec<(Range<usize>, HighlightStyle)> = Vec::new();
    let mut code: Vec<(Range<usize>, SharedString)> = Vec::new();
    let mut links: Vec<(Range<usize>, Action)> = Vec::new();
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
                highlights.push((
                    start..text.len(),
                    HighlightStyle {
                        color: Some(theme.link),
                        ..Default::default()
                    },
                ));
                links.push((start..text.len(), action.clone()));
            }
        }
    }
    let spec = Variant::Body2.spec();
    let styled = StyledText::new(text)
        .with_highlights(highlights)
        .with_font_family_overrides(code);
    let frame = div()
        .text_size(px(spec.size))
        .line_height(px(spec.size * spec.line_height));
    if links.is_empty() {
        return frame.child(styled).into_any_element();
    }
    let (ranges, actions): (Vec<_>, Vec<_>) = links.into_iter().unzip();
    let on_action = page.on_action.clone();
    frame
        .child(
            InteractiveText::new(page.id(&format!("{path}:text")), styled).on_click(
                ranges,
                move |index, window, cx| {
                    if let Some(action) = actions.get(index) {
                        on_action(action, window, cx);
                    }
                },
            ),
        )
        .into_any_element()
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

/// A table outside a page, such as one in an illustration: no links, and
/// named by `key` among the other things drawn that frame.
pub fn plain_table(table: &Table, key: &str, window: &mut Window, cx: &mut App) -> AnyElement {
    let page = Page {
        serial: usize::MAX,
        on_action: Rc::new(|_, _, _| {}),
    };
    draw_table(table, key, &page, window, cx)
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
        .px(px(CELL_PADDING.0))
        .py(px(CELL_PADDING.1))
        .when(column.numeric, |cell| cell.flex().justify_end())
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
                .gap(px(8.))
                .items_center()
                .child(feedback::progress(*share, color, cx).w(px(56.)))
                .child(label.clone())
                .into_any_element()
        }
    }
}

fn draw_row(row: &Row, id: ElementId, page: &Page) -> ListItemButton {
    let on_action = page.on_action.clone();
    let action = row.action.clone();
    ListItemButton::new(id, row.label.clone())
        .icon(row.icon)
        .code(row.code)
        .when_some(row.detail.clone(), |item, detail| item.secondary(detail))
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
    h_flex()
        .gap(px(12.))
        .items_center()
        .child(
            surfaces::text(Variant::Body2, label.to_string())
                .w(px(120.))
                .flex_shrink_0()
                .text_color(cx.theme().muted_foreground),
        )
        .child(
            ToggleButtonGroup::new(format!("doc{}:{path}", page.serial), labels)
                .selected(selected)
                .on_select(move |index, window, cx| {
                    if let Some(action) = actions.get(index) {
                        on_action(action, window, cx);
                    }
                }),
        )
        .into_any_element()
}

/// A theme to pick: a card that can be pressed, marked when it is the one in
/// use.
fn swatch_card(swatch: &Swatch, id: ElementId, page: &Page, cx: &App) -> AnyElement {
    let on_action = page.on_action.clone();
    let action = Action::Theme(swatch.name.clone());
    div()
        .w(px(TILE_MIN + 40.0))
        .child(
            CardButton::new(id)
                .selected(swatch.active)
                .on_click(move |_, window, cx| on_action(&action, window, cx))
                .child(
                    h_flex()
                        .gap(px(8.))
                        .items_center()
                        .child(
                            surfaces::text(Variant::Subtitle2, swatch.name.clone())
                                .flex_1()
                                .min_w_0()
                                .truncate(),
                        )
                        .when(swatch.active, |row| {
                            row.child(
                                Icon::new(icons::CHECK)
                                    .with_size(px(18.))
                                    .text_color(cx.theme().primary),
                            )
                        }),
                ),
        )
        .into_any_element()
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
}
