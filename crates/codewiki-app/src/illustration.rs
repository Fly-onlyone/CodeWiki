//! Native renderer for illustrations.
//!
//! Turns an evaluated OpenUI Lang tree into GPUI elements, drawn with the
//! `mui` components. Every component of the CodeWiki library
//! (`cw_illustrate::library`) has a branch here; a component this renderer
//! does not know is drawn as its name, never dropped silently.

use gpui_kit::component::{ActiveTheme as _, StyledExt as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use openui_lang::Node;
use std::collections::BTreeMap;

use crate::doc;
use crate::doc_view;
use crate::mui::feedback::{self, Tone};
use crate::mui::motion::{Enter, EnterExt as _};
use crate::mui::surfaces::{self, card, card_content};
use crate::mui::tabs::{Tab, Tabs};
use crate::mui::tokens::Variant;

/// MUI's `StepIcon`: a 24 px disc with the step's number in 12 px text.
const STEP_ICON: f32 = 24.0;

type Props = BTreeMap<String, Node>;

fn text_of(props: &Props, key: &str) -> Option<String> {
    match props.get(key)? {
        Node::Text(s) => Some(s.clone()),
        Node::Number(n) if n.fract() == 0.0 => Some(format!("{}", *n as i64)),
        Node::Number(n) => Some(format!("{n}")),
        _ => None,
    }
}

fn list_of<'a>(props: &'a Props, key: &str) -> &'a [Node] {
    props.get(key).map(Node::as_list).unwrap_or(&[])
}

fn numbers(nodes: &[Node]) -> Vec<f64> {
    nodes
        .iter()
        .map(|n| match n {
            Node::Number(x) => *x,
            _ => 0.0,
        })
        .collect()
}

fn label(node: &Node) -> String {
    match node {
        Node::Text(s) => s.clone(),
        Node::Number(n) if n.fract() == 0.0 => format!("{}", *n as i64),
        Node::Number(n) => format!("{n}"),
        Node::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

fn tone_color(tone: Option<&str>, cx: &App) -> Hsla {
    let theme = cx.theme();
    match tone {
        Some("muted") => theme.muted_foreground,
        Some("success") => theme.success,
        Some("warning") => theme.warning,
        Some("danger") => theme.danger,
        _ => theme.foreground,
    }
}

/// Series colors, cycled.
fn series_color(index: usize, cx: &App) -> Hsla {
    let theme = cx.theme();
    [
        theme.primary,
        theme.success,
        theme.warning,
        theme.danger,
        theme.info,
    ][index % 5]
}

/// The tone a component names, as the `mui` components know it.
fn tone_of(tone: Option<&str>) -> Tone {
    match tone {
        Some("success") => Tone::Success,
        Some("warning") => Tone::Warning,
        Some("danger") => Tone::Error,
        _ => Tone::Neutral,
    }
}

/// `path` names each node by its place in the tree, so that a component that
/// remembers something, such as the chosen tab, keeps it between frames.
fn children(nodes: &[Node], path: &str, window: &mut Window, cx: &mut App) -> Vec<AnyElement> {
    nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| !matches!(n, Node::Null))
        .map(|(index, n)| render(n, &format!("{path}.{index}"), window, cx))
        .collect()
}

/// Horizontal bars: one row per label, one bar per series.
fn bar_chart(props: &Props, cx: &App) -> AnyElement {
    let labels: Vec<String> = list_of(props, "labels").iter().map(label).collect();
    let series: Vec<(String, Vec<f64>)> = list_of(props, "series")
        .iter()
        .filter_map(|s| match s {
            Node::Element { props, .. } => Some((
                text_of(props, "name").unwrap_or_default(),
                numbers(list_of(props, "values")),
            )),
            _ => None,
        })
        .collect();
    let max = series
        .iter()
        .flat_map(|(_, values)| values.iter().copied())
        .fold(0.0f64, f64::max)
        .max(1e-9);
    let mut chart = v_flex().gap_2().w_full();
    if let Some(title) = text_of(props, "title") {
        chart = chart.child(div().text_sm().font_semibold().child(title));
    }
    for (row, name) in labels.iter().enumerate() {
        let mut bars = v_flex().flex_1().gap_1();
        for (index, (_, values)) in series.iter().enumerate() {
            let value = values.get(row).copied().unwrap_or(0.0);
            bars = bars.child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .h(px(14.))
                            .rounded(px(3.))
                            .bg(cx.theme().muted)
                            .child(
                                div()
                                    .h_full()
                                    .rounded(px(3.))
                                    .bg(series_color(index, cx))
                                    .w(relative((value / max).clamp(0.0, 1.0) as f32)),
                            ),
                    )
                    .child(
                        div()
                            .w(px(56.))
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(label(&Node::Number(value))),
                    ),
            );
        }
        chart = chart.child(
            h_flex()
                .gap_3()
                .items_center()
                .child(div().w(px(140.)).text_sm().truncate().child(name.clone()))
                .child(bars),
        );
    }
    if series.len() > 1 {
        let mut legend = h_flex().gap_4().pt_1();
        for (index, (name, _)) in series.iter().enumerate() {
            legend = legend.child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(
                        div()
                            .size(px(10.))
                            .rounded(px(2.))
                            .bg(series_color(index, cx)),
                    )
                    .child(div().text_xs().child(name.clone())),
            );
        }
        chart = chart.child(legend);
    }
    chart.into_any_element()
}

/// A pie is drawn as shares: label, proportional bar, percentage.
fn pie_chart(props: &Props, cx: &App) -> AnyElement {
    let labels: Vec<String> = list_of(props, "labels").iter().map(label).collect();
    let values = numbers(list_of(props, "values"));
    let total: f64 = values.iter().sum::<f64>().max(1e-9);
    let mut chart = v_flex().gap_2().w_full();
    if let Some(title) = text_of(props, "title") {
        chart = chart.child(div().text_sm().font_semibold().child(title));
    }
    for (index, name) in labels.iter().enumerate() {
        let share = values.get(index).copied().unwrap_or(0.0) / total;
        chart = chart.child(
            h_flex()
                .gap_3()
                .items_center()
                .child(div().w(px(140.)).text_sm().truncate().child(name.clone()))
                .child(
                    div()
                        .flex_1()
                        .h(px(14.))
                        .rounded(px(3.))
                        .bg(cx.theme().muted)
                        .child(
                            div()
                                .h_full()
                                .rounded(px(3.))
                                .bg(series_color(index, cx))
                                .w(relative(share.clamp(0.0, 1.0) as f32)),
                        ),
                )
                .child(
                    div()
                        .w(px(56.))
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{:.0}%", share * 100.0)),
                ),
        );
    }
    chart.into_any_element()
}

fn table(props: &Props, path: &str, window: &mut Window, cx: &mut App) -> AnyElement {
    let columns: Vec<(String, Vec<String>)> = list_of(props, "columns")
        .iter()
        .filter_map(|c| match c {
            Node::Element { props, .. } => Some((
                text_of(props, "label").unwrap_or_default(),
                list_of(props, "values").iter().map(label).collect(),
            )),
            _ => None,
        })
        .collect();
    let rows = columns
        .iter()
        .map(|(_, values)| values.len())
        .max()
        .unwrap_or(0);
    let mut table = doc::Table::new(
        columns
            .iter()
            .map(|(name, _)| doc::Column::new(name.clone())),
    );
    for row in 0..rows {
        table.row(
            columns
                .iter()
                .map(|(_, values)| doc::Cell::text(values.get(row).cloned().unwrap_or_default())),
        );
    }
    doc_view::plain_table(&table, path, window, cx)
}

/// Tabs: one panel shown at a time, the one whose tab was chosen.
fn tabs(props: &Props, path: &str, window: &mut Window, cx: &mut App) -> AnyElement {
    let items: Vec<&Props> = list_of(props, "items")
        .iter()
        .filter_map(|tab| match tab {
            Node::Element { props, .. } => Some(props),
            _ => None,
        })
        .collect();
    let id = SharedString::from(format!("figure-tabs:{path}"));
    let chosen = window.use_keyed_state(ElementId::Name(id.clone()), cx, |_, _| 0usize);
    let selected = (*chosen.read(cx)).min(items.len().saturating_sub(1));
    let strip = Tabs::new(id.clone())
        .tabs(
            items
                .iter()
                .map(|props| Tab::new(text_of(props, "label").unwrap_or_default())),
        )
        .selected(selected)
        .on_select(move |index, _, cx| {
            chosen.update(cx, |chosen, cx| {
                *chosen = index;
                cx.notify();
            });
        });
    let panel = items.get(selected).map(|props| {
        v_flex()
            .w_full()
            .gap_3()
            .children(children(
                list_of(props, "children"),
                &format!("{path}.{selected}"),
                window,
                cx,
            ))
            // TradingAgent's `tabSwitch`: the new panel rises into place.
            .enter(
                ElementId::NamedInteger(id.clone(), selected as u64),
                Enter::TAB,
            )
    });
    v_flex()
        .w_full()
        .gap_3()
        .child(
            div()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(strip),
        )
        .children(panel)
        .into_any_element()
}

/// MUI's vertical `Stepper`: numbered discs joined by a line.
fn steps(props: &Props, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let items: Vec<&Props> = list_of(props, "items")
        .iter()
        .filter_map(|step| match step {
            Node::Element { props, .. } => Some(props),
            _ => None,
        })
        .collect();
    let last = items.len().saturating_sub(1);
    let mut out = v_flex().w_full();
    for (index, props) in items.into_iter().enumerate() {
        out = out.child(
            h_flex()
                .gap_3()
                .items_stretch()
                .child(
                    v_flex()
                        .flex_shrink_0()
                        .items_center()
                        .child(
                            div()
                                .flex_shrink_0()
                                .size(px(STEP_ICON))
                                .rounded_full()
                                .bg(theme.button_primary)
                                .text_color(theme.button_primary_foreground)
                                .text_size(px(12.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(format!("{}", index + 1)),
                        )
                        // `StepConnector`: a line down to the next disc.
                        .when(index != last, |rail| {
                            rail.child(div().flex_1().min_h(px(16.)).w(px(1.)).bg(theme.border))
                        }),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .pb_3()
                        .child(
                            surfaces::text(
                                Variant::Subtitle2,
                                text_of(props, "title").unwrap_or_default(),
                            )
                            .line_height(px(STEP_ICON)),
                        )
                        .when_some(text_of(props, "detail"), |this, detail| {
                            this.child(
                                surfaces::text(Variant::Body2, detail)
                                    .text_color(theme.muted_foreground),
                            )
                        }),
                ),
        );
    }
    out.into_any_element()
}

/// Draw one node of an illustration. `path` names its place in the tree.
pub fn render(node: &Node, path: &str, window: &mut Window, cx: &mut App) -> AnyElement {
    let Node::Element { name, props } = node else {
        return surfaces::text(Variant::Body2, label(node)).into_any_element();
    };
    match name.as_str() {
        "Card" => {
            let content = card_content()
                .gap_4()
                .when_some(text_of(props, "title"), |this, title| {
                    this.child(surfaces::text(Variant::H4, title))
                })
                .children(children(list_of(props, "children"), path, window, cx));
            card(cx).w_full().child(content).into_any_element()
        }
        "Stack" => {
            let row = text_of(props, "direction").as_deref() == Some("row");
            let stack = if row {
                h_flex().items_start()
            } else {
                v_flex()
            };
            match text_of(props, "gap").as_deref() {
                Some("s") => stack.gap_1(),
                Some("l") => stack.gap_6(),
                _ => stack.gap_3(),
            }
            .w_full()
            .children(children(list_of(props, "children"), path, window, cx))
            .into_any_element()
        }
        "Grid" => {
            let items = children(list_of(props, "children"), path, window, cx);
            let columns = match props.get("columns") {
                Some(Node::Number(n)) if *n >= 1.0 => (*n as usize).min(6),
                _ => items.len().clamp(1, 4),
            };
            let width = relative(1.0 / columns as f32);
            h_flex()
                .w_full()
                .flex_wrap()
                .items_stretch()
                .children(
                    items
                        .into_iter()
                        .map(|item| div().w(width).p_1().child(item)),
                )
                .into_any_element()
        }
        "Tabs" => tabs(props, path, window, cx),
        "Divider" => surfaces::divider(cx).into_any_element(),
        "Header" => v_flex()
            .gap_1()
            .child(surfaces::text(
                Variant::H3,
                text_of(props, "title").unwrap_or_default(),
            ))
            .when_some(text_of(props, "subtitle"), |this, subtitle| {
                this.child(
                    surfaces::text(Variant::Body2, subtitle)
                        .text_color(cx.theme().muted_foreground),
                )
            })
            .into_any_element(),
        "TextContent" => {
            let tone = text_of(props, "tone");
            surfaces::text(Variant::Body2, text_of(props, "text").unwrap_or_default())
                .text_color(tone_color(tone.as_deref(), cx))
                .when(tone.as_deref() == Some("strong"), |this| {
                    this.font_semibold()
                })
                .into_any_element()
        }
        "Stat" => {
            let mut stat = doc::Stat::new(
                text_of(props, "label").unwrap_or_default(),
                text_of(props, "value").unwrap_or_default(),
            );
            if let Some(hint) = text_of(props, "hint") {
                stat = stat.hint(hint);
            }
            doc_view::stat_tile(&stat, cx).into_any_element()
        }
        "Badge" => h_flex()
            .child(feedback::chip(
                text_of(props, "text").unwrap_or_default(),
                tone_of(text_of(props, "tone").as_deref()),
                false,
                cx,
            ))
            .into_any_element(),
        "Steps" => steps(props, cx),
        "Step" => surfaces::text(Variant::Body2, text_of(props, "title").unwrap_or_default())
            .into_any_element(),
        "Table" => table(props, path, window, cx),
        // A line chart is drawn with the same bars: the values are exact,
        // only the shape differs.
        "BarChart" | "LineChart" => bar_chart(props, cx),
        "PieChart" => pie_chart(props, cx),
        other => surfaces::text(Variant::Caption, format!("[{other}]"))
            .text_color(cx.theme().muted_foreground)
            .into_any_element(),
    }
}
