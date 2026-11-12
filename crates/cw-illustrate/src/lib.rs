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

/// An overview card: a title, a one-line summary, headline figures and an
/// optional bar chart. Built from structural facts only, so it needs no LLM.
pub fn overview(
    title: &str,
    summary: &str,
    figures: &[Figure],
    bars: &[(String, f64)],
    bars_title: &str,
) -> String {
    let mut body = vec!["summary".to_string()];
    let mut out = String::new();
    out.push_str(&format!(
        "summary = TextContent({}, \"muted\")\n",
        quote(summary)
    ));
    if !figures.is_empty() {
        let names: Vec<String> = (0..figures.len()).map(|i| format!("f{i}")).collect();
        out.push_str(&format!(
            "figures = Grid([{}], {})\n",
            names.join(", "),
            figures.len().min(4)
        ));
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
        body.push("figures".to_string());
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
        body.push("chart".to_string());
    }
    format!(
        "root = Card([{}], {})\n{out}",
        body.join(", "),
        quote(title)
    )
}

/// A numbered sequence of steps, for flows.
pub fn steps(title: &str, summary: &str, items: &[(String, String)]) -> String {
    let names: Vec<String> = (0..items.len()).map(|i| format!("s{i}")).collect();
    let mut out = format!(
        "root = Card([summary, flow], {})\nsummary = TextContent({}, \"muted\")\nflow = Steps([{}])\n",
        quote(title),
        quote(summary),
        names.join(", ")
    );
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
