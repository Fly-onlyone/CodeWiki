//! Native renderer for the pictures of a note.
//!
//! Turns an evaluated OpenUI Lang tree into GPUI elements, drawn with the
//! `mui` components. Every component of the CodeWiki library
//! (`cw_illustrate::library`) has a branch here; a component this renderer
//! does not know is drawn as its name, never dropped silently.
//!
//! A picture explains what the text beside it says: a `Flow` is a row of
//! numbered boxes joined by arrows, read from a cause to its result, and a
//! `Callout` is a marked key point.

use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use openui_lang::Node;
use std::collections::BTreeMap;

use crate::icons;
use crate::mui::feedback::{self, Tone};
use crate::mui::surfaces::{self, card};
use crate::mui::tokens::Variant;
use crate::zoom::px;

/// MUI's `StepIcon`: a 24 px disc with the step's number in 12 px text.
const STEP_ICON: f32 = 24.0;
/// The narrowest a step's box gets before its row wraps. Three boxes of
/// this width and their arrows fit the note's column.
const STEP_MIN: f32 = 168.0;
/// Boxes in one row at most. A longer flow takes several rows.
const ROW_STEPS: usize = 3;
/// The arrow that leads to a step, and the room it stands in.
const ARROW: f32 = 18.0;
const ARROW_SLOT: f32 = 28.0;
/// Space between two rows of a flow that wrapped.
const ROW_GAP: f32 = 8.0;
/// Inside a step's box.
const STEP_PADDING: f32 = 12.0;

type Props = BTreeMap<String, Node>;

fn text_of(props: &Props, key: &str) -> Option<String> {
    match props.get(key)? {
        Node::Text(s) if !s.trim().is_empty() => Some(s.clone()),
        _ => None,
    }
}

fn list_of<'a>(props: &'a Props, key: &str) -> &'a [Node] {
    props.get(key).map(Node::as_list).unwrap_or(&[])
}

/// The tone a callout names, as the `mui` components know it. One without a
/// tone is a plain piece of information.
fn tone_of(tone: Option<&str>) -> Tone {
    match tone {
        Some("success") => Tone::Success,
        Some("warning") => Tone::Warning,
        Some("danger") => Tone::Error,
        _ => Tone::Info,
    }
}

/// One box of a flow: its number, its title, and a sentence under them.
fn step(number: usize, props: &Props, cx: &App) -> Div {
    let theme = cx.theme();
    card(cx).flex_1().min_w_0().child(
        v_flex()
            .p(px(STEP_PADDING))
            .gap_1()
            .child(
                h_flex()
                    .gap_2()
                    .items_start()
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
                            .child(format!("{number}")),
                    )
                    .child(
                        div().flex_1().min_w_0().child(
                            surfaces::text(
                                Variant::Subtitle2,
                                text_of(props, "title").unwrap_or_default(),
                            )
                            // The first line stands level with the number.
                            .line_height(px(STEP_ICON)),
                        ),
                    ),
            )
            .when_some(text_of(props, "text"), |this, text| {
                this.child(surfaces::text(Variant::Body2, text).text_color(theme.muted_foreground))
            }),
    )
}

/// How many boxes stand in each row of a flow of `steps` boxes: as few
/// rows as [`ROW_STEPS`] allows, and the boxes spread evenly over them, so
/// that no row ends with one box alone. Four steps are two rows of two.
fn rows(steps: usize) -> Vec<usize> {
    if steps == 0 {
        return Vec::new();
    }
    let rows = steps.div_ceil(ROW_STEPS);
    (0..rows)
        .map(|row| steps / rows + usize::from(row < steps % rows))
        .collect()
}

/// A flow: boxes in rows, each box but the last followed by the arrow that
/// leads on from it. The arrow after a row's last box says that the flow
/// goes on in the next row; the numbers carry the order. Every box has the
/// room of an arrow after it, so the boxes of all rows stand in line. In a
/// pane narrower than the note's column a row wraps by itself.
fn flow(props: &Props, cx: &App) -> AnyElement {
    let steps: Vec<&Props> = list_of(props, "steps")
        .iter()
        .filter_map(|step| match step {
            Node::Element { props, .. } => Some(props),
            _ => None,
        })
        .collect();
    let muted = cx.theme().muted_foreground;
    let last = steps.len().saturating_sub(1);
    let boxed = |index: usize, props: &Props| {
        h_flex()
            .items_stretch()
            .flex_grow(1.)
            .flex_basis(px(STEP_MIN + ARROW_SLOT))
            .min_w(px(STEP_MIN + ARROW_SLOT))
            .child(step(index + 1, props, cx))
            .child(
                h_flex()
                    .flex_shrink_0()
                    .w(px(ARROW_SLOT))
                    .items_center()
                    .justify_center()
                    .when(index < last, |slot| {
                        slot.child(
                            Icon::new(icons::FORWARD)
                                .with_size(px(ARROW))
                                .text_color(muted),
                        )
                    }),
            )
    };
    let mut body = v_flex()
        .w_full()
        .gap(px(ROW_GAP))
        .when_some(text_of(props, "title"), |this, title| {
            this.child(surfaces::text(Variant::Subtitle2, title).text_color(muted))
        });
    let mut first = 0;
    for count in rows(steps.len()) {
        body = body.child(
            h_flex()
                .w_full()
                .flex_wrap()
                .items_stretch()
                .gap_y(px(ROW_GAP))
                .children(
                    steps[first..first + count]
                        .iter()
                        .enumerate()
                        .map(|(offset, props)| boxed(first + offset, props)),
                ),
        );
        first += count;
    }
    body.into_any_element()
}

/// Draw one node of a picture.
pub fn render(node: &Node, cx: &App) -> AnyElement {
    let Node::Element { name, props } = node else {
        return div().into_any_element();
    };
    match name.as_str() {
        "Picture" => v_flex()
            .w_full()
            .gap_3()
            .children(
                list_of(props, "children")
                    .iter()
                    .filter(|child| !matches!(child, Node::Null))
                    .map(|child| render(child, cx)),
            )
            .into_any_element(),
        "Flow" => flow(props, cx),
        // A step outside a flow: its box alone.
        "Step" => step(1, props, cx).into_any_element(),
        "Callout" => feedback::alert_titled(
            tone_of(text_of(props, "tone").as_deref()),
            text_of(props, "title"),
            text_of(props, "text").unwrap_or_default(),
            cx,
        )
        .into_any_element(),
        other => surfaces::text(Variant::Caption, format!("[{other}]"))
            .text_color(cx.theme().muted_foreground)
            .into_any_element(),
    }
}

#[cfg(test)]
mod tests {
    // Named one by one: the glob above also brings in the kit's own `test`
    // attribute, which would take the place of the standard one here.
    use super::{ARROW_SLOT, ROW_STEPS, STEP_MIN, Tone, rows, tone_of};

    #[test]
    fn every_component_of_the_library_is_drawn() {
        // `render` matches on names. A component added to the library with
        // no branch here would be drawn as its name in brackets.
        let drawn = ["Picture", "Flow", "Step", "Callout"];
        let library = cw_illustrate::library();
        let mut names: Vec<&str> = library.names().collect();
        names.sort();
        let mut expected = drawn.to_vec();
        expected.sort();
        assert_eq!(names, expected);
    }

    #[test]
    fn a_callout_without_a_tone_is_information() {
        assert_eq!(tone_of(None), Tone::Info);
        assert_eq!(tone_of(Some("info")), Tone::Info);
        assert_eq!(tone_of(Some("warning")), Tone::Warning);
        assert_eq!(tone_of(Some("success")), Tone::Success);
        assert_eq!(tone_of(Some("danger")), Tone::Error);
    }

    #[test]
    fn a_full_row_fits_the_column_of_a_note() {
        // Each box has the room of an arrow after it.
        let row = ROW_STEPS as f32 * (STEP_MIN + ARROW_SLOT);
        assert!(row <= crate::reading::NOTE_MEASURE, "{row}");
    }

    #[test]
    fn a_flow_spreads_its_boxes_evenly_over_its_rows() {
        assert_eq!(rows(2), [2]);
        assert_eq!(rows(3), [3]);
        // Not three and one.
        assert_eq!(rows(4), [2, 2]);
        assert_eq!(rows(5), [3, 2]);
        assert_eq!(rows(6), [3, 3]);
        assert_eq!(rows(7), [3, 2, 2]);
        assert!(rows(0).is_empty());
        for steps in 1..=cw_illustrate::MAX_STEPS {
            let rows = rows(steps);
            assert_eq!(rows.iter().sum::<usize>(), steps);
            assert!(rows.iter().all(|count| (1..=ROW_STEPS).contains(count)));
        }
    }
}
