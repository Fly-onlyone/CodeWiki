//! CodeWiki's illustration component library and structural builders.
//!
//! An illustration is an OpenUI Lang program saved beside a note as
//! `<Note>.openui`. The app draws it natively; nothing is fetched or executed.
//! This crate is the single definition of which components exist, so the
//! generator, the validator and the renderer cannot drift apart.

use openui_lang::{Document, Kind, Library, evaluate, parse};

/// File extension of saved illustrations.
pub const EXTENSION: &str = "openui";

/// The components an illustration may use.
///
/// Layout: `Card`, `Stack`, `Grid`, `Tabs`/`Tab`, `Divider`.
/// Content: `Header`, `TextContent`, `Stat`, `Badge`, `Steps`/`Step`,
/// `Table`/`Col`. Charts: `BarChart`, `LineChart`, `PieChart` with `Series`.
pub fn library() -> Library {
    use Kind::{List, Number, Text};
    Library::new(Some("Card"))
        .component("Card", &[("children", List, true), ("title", Text, false)])
        // direction: "row" or "column" (default). gap: "s", "m" (default) or "l".
        .component(
            "Stack",
            &[
                ("children", List, true),
                ("direction", Text, false),
                ("gap", Text, false),
            ],
        )
        .component(
            "Grid",
            &[("children", List, true), ("columns", Number, false)],
        )
        .component("Tabs", &[("items", List, true)])
        .component("Tab", &[("label", Text, true), ("children", List, true)])
        .component("Divider", &[])
        .component(
            "Header",
            &[("title", Text, true), ("subtitle", Text, false)],
        )
        // tone: "default", "muted", "strong", "success", "warning" or "danger".
        .component(
            "TextContent",
            &[("text", Text, true), ("tone", Text, false)],
        )
        .component(
            "Stat",
            &[
                ("label", Text, true),
                ("value", Text, true),
                ("hint", Text, false),
            ],
        )
        .component("Badge", &[("text", Text, true), ("tone", Text, false)])
        .component("Steps", &[("items", List, true)])
        .component("Step", &[("title", Text, true), ("detail", Text, false)])
        .component("Table", &[("columns", List, true)])
        .component("Col", &[("label", Text, true), ("values", List, true)])
        .component(
            "BarChart",
            &[
                ("labels", List, true),
                ("series", List, true),
                ("title", Text, false),
            ],
        )
        .component(
            "LineChart",
            &[
                ("labels", List, true),
                ("series", List, true),
                ("title", Text, false),
            ],
        )
        .component(
            "PieChart",
            &[
                ("labels", List, true),
                ("values", List, true),
                ("title", Text, false),
            ],
        )
        .component("Series", &[("name", Text, true), ("values", List, true)])
}

/// Parse and check an illustration against the library.
pub fn check(source: &str) -> Document {
    evaluate(&parse(source), &library())
}

/// Quote text as an OpenUI Lang string literal.
pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push(' '),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// One headline figure.
#[derive(Debug, Clone, PartialEq)]
pub struct Figure {
    pub label: String,
    pub value: String,
    pub hint: Option<String>,
}

/// The frame of an illustration: a card around the parts named in `body`.
/// An empty `title` gives a card without one, for an illustration that
/// stands under its note's own title.
fn card(body: &[&str], title: &str) -> String {
    if title.is_empty() {
        format!("root = Card([{}])\n", body.join(", "))
    } else {
        format!("root = Card([{}], {})\n", body.join(", "), quote(title))
    }
}

/// The assignments of a row of headline figures, named `figures`.
fn figure_row(figures: &[Figure]) -> String {
    let names: Vec<String> = (0..figures.len()).map(|i| format!("f{i}")).collect();
    // An illustration stands in a note's column, where four figures side
    // by side are too narrow to read: four make two rows of two.
    let columns = match figures.len() {
        4 => 2,
        n => n.min(3),
    };
    let mut out = format!("figures = Grid([{}], {columns})\n", names.join(", "));
    for (name, figure) in names.iter().zip(figures) {
        let hint = figure
            .hint
            .as_deref()
            .map(|h| format!(", {}", quote(h)))
            .unwrap_or_default();
        out.push_str(&format!(
            "{name} = Stat({}, {}{hint})\n",
            quote(&figure.label),
            quote(&figure.value)
        ));
    }
    out
}

/// The assignments of a numbered sequence, named `flow`.
fn step_list(items: &[(String, String)]) -> String {
    let names: Vec<String> = (0..items.len()).map(|i| format!("s{i}")).collect();
    let mut out = format!("flow = Steps([{}])\n", names.join(", "));
    for (name, (step, detail)) in names.iter().zip(items) {
        if detail.is_empty() {
            out.push_str(&format!("{name} = Step({})\n", quote(step)));
        } else {
            out.push_str(&format!(
                "{name} = Step({}, {})\n",
                quote(step),
                quote(detail)
            ));
        }
    }
    out
}

/// An overview card: headline figures and an optional bar chart, under an
/// optional title and one-line summary. Built from structural facts only,
/// so it needs no LLM. Leave `title` and `summary` empty for an illustration
/// shown under its note's own.
pub fn overview(
    title: &str,
    summary: &str,
    figures: &[Figure],
    bars: &[(String, f64)],
    bars_title: &str,
) -> String {
    let mut body = Vec::new();
    let mut out = String::new();
    if !summary.is_empty() {
        out.push_str(&format!(
            "summary = TextContent({}, \"muted\")\n",
            quote(summary)
        ));
        body.push("summary");
    }
    if !figures.is_empty() {
        out.push_str(&figure_row(figures));
        body.push("figures");
    }
    if bars.len() >= 2 {
        let labels: Vec<String> = bars.iter().map(|(label, _)| quote(label)).collect();
        let values: Vec<String> = bars.iter().map(|(_, value)| number(*value)).collect();
        out.push_str(&format!(
            "chart = BarChart([{}], [Series({}, [{}])], {})\n",
            labels.join(", "),
            quote(bars_title),
            values.join(", "),
            quote(bars_title)
        ));
        body.push("chart");
    }
    format!("{}{out}", card(&body, title))
}

/// A numbered sequence of steps, for flows, under an optional title and
/// one-line summary.
pub fn steps(title: &str, summary: &str, items: &[(String, String)]) -> String {
    let mut body = Vec::new();
    let mut out = String::new();
    if !summary.is_empty() {
        out.push_str(&format!(
            "summary = TextContent({}, \"muted\")\n",
            quote(summary)
        ));
        body.push("summary");
    }
    out.push_str(&step_list(items));
    body.push("flow");
    format!("{}{out}", card(&body, title))
}

/// What a note is about at a glance: headline figures, then the steps of
/// its walk-through. It has no title and no summary, since it stands under
/// the note's own. `None` when there is neither a figure nor a step.
pub fn brief(figures: &[Figure], steps: &[(String, String)]) -> Option<String> {
    let mut body = Vec::new();
    let mut out = String::new();
    if !figures.is_empty() {
        out.push_str(&figure_row(figures));
        body.push("figures");
    }
    if !steps.is_empty() {
        out.push_str(&step_list(steps));
        body.push("flow");
    }
    (!body.is_empty()).then(|| format!("{}{out}", card(&body, "")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_lang::Node;

    fn props(node: &Node) -> &std::collections::BTreeMap<String, Node> {
        match node {
            Node::Element { props, .. } => props,
            other => panic!("expected an element, got {other:?}"),
        }
    }

    #[test]
    fn overview_builds_a_valid_illustration() {
        let source = overview(
            "cw-core",
            "Shared \"types\" and\nhashing.",
            &[
                Figure {
                    label: "Modules".into(),
                    value: "4".into(),
                    hint: None,
                },
                Figure {
                    label: "Used by".into(),
                    value: "9 crates".into(),
                    hint: Some("most depended upon".into()),
                },
            ],
            &[("hash".into(), 3.0), ("model".into(), 12.5)],
            "Symbols per module",
        );
        let doc = check(&source);
        assert!(doc.is_valid(), "{:?}\n{source}", doc.errors);
        assert!(doc.errors.is_empty(), "{:?}", doc.errors);
        let card = props(doc.root.as_ref().unwrap());
        assert_eq!(card["title"].as_text(), Some("cw-core"));
        let children = card["children"].as_list();
        assert_eq!(children.len(), 3);
        assert_eq!(
            props(&children[0])["text"].as_text(),
            Some("Shared \"types\" and\nhashing.")
        );
        let figures = props(&children[1])["children"].as_list();
        assert_eq!(
            props(&figures[1])["hint"].as_text(),
            Some("most depended upon")
        );
        let series = props(&children[2])["series"].as_list();
        assert_eq!(
            props(&series[0])["values"].as_list(),
            [Node::Number(3.0), Node::Number(12.5)]
        );
    }

    #[test]
    fn overview_omits_empty_parts() {
        let doc = check(&overview(
            "x",
            "Only a summary.",
            &[],
            &[("one".into(), 1.0)],
            "n",
        ));
        assert!(doc.is_valid());
        assert_eq!(
            props(doc.root.as_ref().unwrap())["children"]
                .as_list()
                .len(),
            1
        );
    }

    #[test]
    fn steps_builds_a_valid_illustration() {
        let source = steps(
            "Startup",
            "What happens at launch.",
            &[
                ("main".into(), "parses arguments".into()),
                ("serve".into(), String::new()),
            ],
        );
        let doc = check(&source);
        assert!(doc.is_valid() && doc.errors.is_empty(), "{:?}", doc.errors);
        let flow = props(&props(doc.root.as_ref().unwrap())["children"].as_list()[1])["items"]
            .as_list()
            .to_vec();
        assert_eq!(
            props(&flow[0])["detail"].as_text(),
            Some("parses arguments")
        );
        assert!(!props(&flow[1]).contains_key("detail"));
    }

    #[test]
    fn brief_stands_under_a_notes_own_title() {
        let figure = |label: &str, value: &str| Figure {
            label: label.into(),
            value: value.into(),
            hint: None,
        };
        let source = brief(
            &[figure("Public items", "4"), figure("Used by notes", "1")],
            &[
                ("enabled".into(), "Calls installed_dir, then read".into()),
                ("set".into(), String::new()),
            ],
        )
        .unwrap();
        assert!(
            source.starts_with("root = Card([figures, flow])\n"),
            "{source}"
        );
        let doc = check(&source);
        assert!(doc.is_valid() && doc.errors.is_empty(), "{:?}", doc.errors);
        let card = props(doc.root.as_ref().unwrap());
        assert!(!card.contains_key("title"));
        assert_eq!(card["children"].as_list().len(), 2);
        // Either part alone, and nothing at all.
        let only_steps = brief(&[], &[("run".into(), String::new())]).unwrap();
        assert!(only_steps.starts_with("root = Card([flow])\n"));
        assert!(check(&only_steps).is_valid());
        assert!(check(&brief(&[figure("Version", "1")], &[]).unwrap()).is_valid());
        assert_eq!(brief(&[], &[]), None);
    }

    #[test]
    fn figures_keep_to_the_width_of_a_note() {
        let figure = |n: usize| Figure {
            label: format!("Figure {n}"),
            value: n.to_string(),
            hint: None,
        };
        let columns = |count: usize| {
            let figures: Vec<Figure> = (0..count).map(figure).collect();
            let source = brief(&figures, &[]).unwrap();
            let grid = source.lines().nth(1).unwrap().to_string();
            grid.rsplit(", ")
                .next()
                .unwrap()
                .trim_end_matches(')')
                .to_string()
        };
        assert_eq!(columns(1), "1");
        assert_eq!(columns(3), "3");
        // Never four in a row.
        assert_eq!(columns(4), "2");
        assert_eq!(columns(5), "3");
    }

    #[test]
    fn a_card_under_a_title_repeats_neither_title_nor_summary() {
        let source = overview("", "", &[], &[("a".into(), 1.0), ("b".into(), 2.0)], "n");
        assert!(source.starts_with("root = Card([chart])\n"), "{source}");
        assert!(check(&source).is_valid());
        let source = steps("", "", &[("main".into(), String::new())]);
        assert!(source.starts_with("root = Card([flow])\n"), "{source}");
        assert!(check(&source).is_valid());
    }

    #[test]
    fn check_rejects_components_outside_the_library() {
        let doc = check("root = Card([x])\nx = Iframe(\"https://example.com\")\n");
        assert!(!doc.is_valid());
        assert_eq!(doc.errors[0].code, "unknown-component");
    }

    #[test]
    fn library_lists_every_component() {
        let names: Vec<&str> = library().names().collect();
        assert_eq!(names.len(), 18);
        for name in [
            "Card", "Stat", "Steps", "Table", "BarChart", "PieChart", "Tabs",
        ] {
            assert!(names.contains(&name), "{name}");
        }
    }
}
