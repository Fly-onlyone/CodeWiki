//! Native renderer for illustrations.
//!
//! Turns an evaluated OpenUI Lang tree into GPUI elements. Every component of
//! the CodeWiki library (`cw_illustrate::library`) has a branch here; a
//! component this renderer does not know is drawn as its name, never dropped
//! silently.

use gpui_kit::component::{ActiveTheme as _, StyledExt as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use openui_lang::Node;
use std::collections::BTreeMap;

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

fn children(nodes: &[Node], cx: &App) -> Vec<AnyElement> {
    nodes
        .iter()
        .filter(|n| !matches!(n, Node::Null))
        .map(|n| render(n, cx))
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

fn table(props: &Props, cx: &App) -> AnyElement {
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
    let mut table = v_flex()
        .w_full()
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius);
    let mut header = h_flex().bg(cx.theme().muted);
    for (name, _) in &columns {
        header = header.child(
            div()
                .flex_1()
                .px_3()
                .py_1()
                .text_sm()
                .font_semibold()
                .child(name.clone()),
        );
    }
    table = table.child(header);
    for row in 0..rows {
        let mut line = h_flex().border_t_1().border_color(cx.theme().border);
        for (_, values) in &columns {
            line = line.child(
                div()
                    .flex_1()
                    .px_3()
                    .py_1()
                    .text_sm()
                    .child(values.get(row).cloned().unwrap_or_default()),
            );
        }
        table = table.child(line);
    }
    table.into_any_element()
}

/// Draw one node of an illustration.
pub fn render(node: &Node, cx: &App) -> AnyElement {
    let Node::Element { name, props } = node else {
        return div().text_sm().child(label(node)).into_any_element();
    };
    let theme = cx.theme();
    match name.as_str() {
        "Card" => v_flex()
            .w_full()
            .gap_4()
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius)
            .when_some(text_of(props, "title"), |this, title| {
                this.child(div().text_lg().font_semibold().child(title))
            })
            .children(children(list_of(props, "children"), cx))
            .into_any_element(),
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
            .children(children(list_of(props, "children"), cx))
            .into_any_element()
        }
        "Grid" => {
            let items = children(list_of(props, "children"), cx);
            let columns = match props.get("columns") {
                Some(Node::Number(n)) if *n >= 1.0 => (*n as usize).min(6),
                _ => items.len().clamp(1, 4),
            };
            let width = relative(1.0 / columns as f32);
            h_flex()
                .w_full()
                .flex_wrap()
                .items_start()
                .children(
                    items
                        .into_iter()
                        .map(|item| div().w(width).p_1().child(item)),
                )
                .into_any_element()
        }
        "Tabs" => {
            // Static documents show every tab, one under another.
            let mut out = v_flex().w_full().gap_4();
            for tab in list_of(props, "items") {
                if let Node::Element { props, .. } = tab {
                    out = out.child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(theme.primary)
                                    .child(text_of(props, "label").unwrap_or_default()),
                            )
                            .children(children(list_of(props, "children"), cx)),
                    );
                }
            }
            out.into_any_element()
        }
        "Divider" => div().w_full().h(px(1.)).bg(theme.border).into_any_element(),
        "Header" => v_flex()
            .gap_1()
            .child(
                div()
                    .text_xl()
                    .font_bold()
                    .child(text_of(props, "title").unwrap_or_default()),
            )
            .when_some(text_of(props, "subtitle"), |this, subtitle| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(subtitle),
                )
            })
            .into_any_element(),
        "TextContent" => {
            let tone = text_of(props, "tone");
            div()
                .text_sm()
                .text_color(tone_color(tone.as_deref(), cx))
                .when(tone.as_deref() == Some("strong"), |this| {
                    this.font_semibold()
                })
                .child(text_of(props, "text").unwrap_or_default())
                .into_any_element()
        }
        "Stat" => v_flex()
            .gap_1()
            .p_3()
            .rounded(theme.radius)
            .bg(theme.muted)
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(text_of(props, "label").unwrap_or_default()),
            )
            .child(
                div()
                    .text_2xl()
                    .font_bold()
                    .child(text_of(props, "value").unwrap_or_default()),
            )
            .when_some(text_of(props, "hint"), |this, hint| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(hint),
                )
            })
            .into_any_element(),
        "Badge" => {
            let tone = text_of(props, "tone");
            h_flex()
                .child(
                    div()
                        .px_2()
                        .rounded(px(999.))
                        .border_1()
                        .border_color(tone_color(tone.as_deref(), cx))
                        .text_xs()
                        .text_color(tone_color(tone.as_deref(), cx))
                        .child(text_of(props, "text").unwrap_or_default()),
                )
                .into_any_element()
        }
        "Steps" => {
            let mut out = v_flex().w_full().gap_3();
            for (index, step) in list_of(props, "items").iter().enumerate() {
                let Node::Element { props, .. } = step else {
                    continue;
                };
                out = out.child(
                    h_flex()
                        .gap_3()
                        .items_start()
                        .child(
                            div()
                                .flex_shrink_0()
                                .size(px(22.))
                                .rounded(px(999.))
                                .bg(theme.primary)
                                .text_color(theme.primary_foreground)
                                .text_xs()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(format!("{}", index + 1)),
                        )
                        .child(
                            v_flex()
                                .flex_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_semibold()
                                        .child(text_of(props, "title").unwrap_or_default()),
                                )
                                .when_some(text_of(props, "detail"), |this, detail| {
                                    this.child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child(detail),
                                    )
                                }),
                        ),
                );
            }
            out.into_any_element()
        }
        "Step" => div()
            .text_sm()
            .child(text_of(props, "title").unwrap_or_default())
            .into_any_element(),
        "Table" => table(props, cx),
        // A line chart is drawn with the same bars: the values are exact,
        // only the shape differs.
        "BarChart" | "LineChart" => bar_chart(props, cx),
        "PieChart" => pie_chart(props, cx),
        other => div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(format!("[{other}]"))
            .into_any_element(),
    }
}
