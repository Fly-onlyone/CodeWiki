//! The pictures of a note: which components exist, what a picture may hold,
//! and the file that keeps a note's pictures.
//!
//! A picture explains what a note says: what happens, from a cause to its
//! result, in plain words. It is written by a coding agent, as an OpenUI
//! Lang program, and the app draws it natively; nothing is fetched or
//! executed. Rules draw none: they know counts and calls, not what a thing
//! is for.
//!
//! This crate is the single definition of the components, so the check a
//! submission must pass, the file the backend keeps and the app's renderer
//! cannot drift apart.

use openui_lang::{Document, Kind, Library, Node, Severity, evaluate, parse};
use serde::{Deserialize, Serialize};

/// File extension of the file that keeps a note's pictures.
pub const EXTENSION: &str = "openui";

/// Steps a flow has at least: one box explains nothing.
pub const MIN_STEPS: usize = 2;
/// Steps a flow has at most. More do not fit a reader's eye, nor the
/// note's column.
pub const MAX_STEPS: usize = 7;
/// Characters of a step's title.
pub const MAX_STEP_TITLE: usize = 40;
/// Characters of a step's text.
pub const MAX_STEP_TEXT: usize = 160;
/// Characters of a callout's text.
pub const MAX_CALLOUT_TEXT: usize = 240;
/// Callouts in one picture.
pub const MAX_CALLOUTS: usize = 2;
/// Bytes of one picture's source.
pub const MAX_SOURCE: usize = 4000;
/// Pictures in one note.
pub const MAX_PICTURES: usize = 4;
/// What a callout can be marked as.
pub const TONES: [&str; 4] = ["info", "warning", "success", "danger"];

/// The components a picture may use.
///
/// - `Picture(children)`: the picture. Its children are at most one `Flow`
///   and at most [`MAX_CALLOUTS`] `Callout`s.
/// - `Flow(steps, title?)`: boxes joined by arrows, read in order.
/// - `Step(title, text?)`: one box of a flow.
/// - `Callout(text, tone?, title?)`: a marked key point or warning; `tone`
///   is one of [`TONES`], `info` when left out.
pub fn library() -> Library {
    use Kind::{List, Text};
    Library::new(Some("Picture"))
        .component("Picture", &[("children", List, true)])
        .component("Flow", &[("steps", List, true), ("title", Text, false)])
        .component("Step", &[("title", Text, true), ("text", Text, false)])
        .component(
            "Callout",
            &[
                ("text", Text, true),
                ("tone", Text, false),
                ("title", Text, false),
            ],
        )
}

/// Parse a picture and check it against the library.
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

/// The limits of a picture, in words: for whoever writes one.
pub fn limits() -> String {
    format!(
        "At most {MAX_PICTURES} pictures in a note, one for a section. A picture holds at most one Flow and at most {MAX_CALLOUTS} Callouts. A Flow has {MIN_STEPS} to {MAX_STEPS} steps. A step's title has at most {MAX_STEP_TITLE} characters and its text at most {MAX_STEP_TEXT}. A callout's text has at most {MAX_CALLOUT_TEXT} characters; its tone is one of {}. Plain words only: no backticks and no links. A picture's source has at most {MAX_SOURCE} bytes.",
        TONES.join(", ")
    )
}

/// The properties of a component, by name.
type Props = std::collections::BTreeMap<String, Node>;

fn element(node: &Node) -> Option<(&str, &Props)> {
    match node {
        Node::Element { name, props } => Some((name.as_str(), props)),
        _ => None,
    }
}

/// The text of the property `key`, and nothing for one left out.
fn text<'a>(props: &'a Props, key: &str) -> &'a str {
    props.get(key).and_then(Node::as_text).unwrap_or("")
}

/// Say what is wrong with `value`, the `what` of a component, when it is
/// missing where it must stand or longer than `most` characters.
fn sized(issues: &mut Vec<String>, what: &str, value: &str, required: bool, most: usize) {
    let length = value.chars().count();
    if required && value.trim().is_empty() {
        issues.push(format!("{what} is empty"));
    } else if length > most {
        issues.push(format!(
            "{what} has {length} characters; at most {most}: \"{}…\"",
            value.chars().take(30).collect::<String>()
        ));
    }
}

/// What is wrong with a picture, one line for each fault; empty when the
/// picture may be kept. It must parse, use only the library's components,
/// and stay within the limits: a picture is a few boxes of plain words, and
/// one that is larger explains less.
pub fn review(source: &str) -> Vec<String> {
    let mut issues = Vec::new();
    if source.len() > MAX_SOURCE {
        issues.push(format!(
            "the source has {} bytes; at most {MAX_SOURCE}",
            source.len()
        ));
    }
    // The app draws plain text: a backtick or a link would show as written.
    if source.contains('`') {
        issues.push("a picture holds plain words: remove the backticks".to_string());
    }
    if source.contains("[[") {
        issues.push("a picture holds no link: remove the [[...]]".to_string());
    }
    // Such a line would start another picture in the file that keeps them.
    if source
        .lines()
        .any(|line| line.trim_start().starts_with(SECTION_MARK.trim_end()))
    {
        issues.push(format!(
            "no line of a picture may start with `{}`",
            SECTION_MARK.trim_end()
        ));
    }
    let document = check(source);
    for error in &document.errors {
        if error.severity == Severity::Error {
            issues.push(format!("{}: {}", error.code, error.message));
        }
    }
    let Some((name, props)) = document.root.as_ref().and_then(element) else {
        if issues.is_empty() {
            issues.push("no picture: assign `root = Picture([...])`".to_string());
        }
        return issues;
    };
    if name != "Picture" {
        issues.push(format!("`root` must be a Picture, not a {name}"));
        return issues;
    }
    let children = props.get("children").map(Node::as_list).unwrap_or(&[]);
    if children.is_empty() {
        issues.push("the picture is empty: give it a Flow or a Callout".to_string());
    }
    let (mut flows, mut callouts) = (0, 0);
    for child in children {
        match element(child) {
            Some(("Flow", flow)) => {
                flows += 1;
                sized(
                    &mut issues,
                    "the flow's title",
                    text(flow, "title"),
                    false,
                    MAX_STEP_TITLE,
                );
                let steps = flow.get("steps").map(Node::as_list).unwrap_or(&[]);
                if !(MIN_STEPS..=MAX_STEPS).contains(&steps.len()) {
                    issues.push(format!(
                        "the flow has {} steps; it takes {MIN_STEPS} to {MAX_STEPS}",
                        steps.len()
                    ));
                }
                for (index, step) in steps.iter().enumerate() {
                    let what = format!("step {}", index + 1);
                    match element(step) {
                        Some(("Step", step)) => {
                            sized(
                                &mut issues,
                                &format!("the title of {what}"),
                                text(step, "title"),
                                true,
                                MAX_STEP_TITLE,
                            );
                            sized(
                                &mut issues,
                                &format!("the text of {what}"),
                                text(step, "text"),
                                false,
                                MAX_STEP_TEXT,
                            );
                        }
                        _ => issues.push(format!("{what} of the flow is not a Step")),
                    }
                }
            }
            Some(("Callout", callout)) => {
                callouts += 1;
                sized(
                    &mut issues,
                    "a callout's text",
                    text(callout, "text"),
                    true,
                    MAX_CALLOUT_TEXT,
                );
                sized(
                    &mut issues,
                    "a callout's title",
                    text(callout, "title"),
                    false,
                    MAX_STEP_TITLE,
                );
                let tone = text(callout, "tone");
                if !tone.is_empty() && !TONES.contains(&tone) {
                    issues.push(format!(
                        "a callout's tone is \"{tone}\"; use {}",
                        TONES.join(", ")
                    ));
                }
            }
            Some((other, _)) => issues.push(format!(
                "a Picture holds Flow and Callout only, not a {other}"
            )),
            None => issues.push("a Picture holds Flow and Callout only".to_string()),
        }
    }
    if flows > 1 {
        issues.push(format!("the picture has {flows} flows; at most one"));
    }
    if callouts > MAX_CALLOUTS {
        issues.push(format!(
            "the picture has {callouts} callouts; at most {MAX_CALLOUTS}"
        ));
    }
    issues
}

/// One picture of a note: the `## ` section it explains, and its source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Picture {
    /// Heading of the section the picture stands in, without the `## `.
    pub section: String,
    /// The picture, as an OpenUI Lang program.
    pub source: String,
}

/// What a line that starts a picture in a pictures file begins with. OpenUI
/// Lang reads a line that starts with `#` as a comment, so each picture of
/// the file is a program by itself, and so is the file.
const SECTION_MARK: &str = "# section: ";

/// The pictures a pictures file holds, in its order. Text before the first
/// mark belongs to no section and is left out.
pub fn parse_pictures(file: &str) -> Vec<Picture> {
    let mut pictures: Vec<Picture> = Vec::new();
    for line in file.lines() {
        if let Some(section) = line.strip_prefix(SECTION_MARK) {
            pictures.push(Picture {
                section: section.trim().to_string(),
                source: String::new(),
            });
        } else if let Some(picture) = pictures.last_mut() {
            picture.source.push_str(line);
            picture.source.push('\n');
        }
    }
    for picture in &mut pictures {
        picture.source = format!("{}\n", picture.source.trim());
    }
    pictures.retain(|picture| !picture.section.is_empty() && !picture.source.trim().is_empty());
    pictures
}

/// The pictures file of `pictures`: each under a line that names its
/// section. Empty for no pictures, in which case no file is kept.
pub fn serialize_pictures(pictures: &[Picture]) -> String {
    pictures
        .iter()
        .map(|picture| {
            format!(
                "{SECTION_MARK}{}\n{}\n",
                picture.section.trim(),
                picture.source.trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The picture of a module that starts an app at sign-in.
    const AUTOSTART: &str = "root = Picture([flow, limit])\nflow = Flow([a, b, c, d])\na = Step(\"You turn it on\", \"The switch Start with Windows in Settings.\")\nb = Step(\"One value is written\", \"The program's path, under the user's Run key.\")\nc = Step(\"You sign in\")\nd = Step(\"The app starts\", \"In the tray, with no window.\")\nlimit = Callout(\"Only an installed copy offers this.\", \"warning\")\n";

    #[test]
    fn a_flow_with_a_callout_is_a_picture() {
        assert_eq!(review(AUTOSTART), Vec::<String>::new());
        let document = check(AUTOSTART);
        assert!(document.is_valid(), "{:?}", document.errors);
        let (name, props) = element(document.root.as_ref().unwrap()).unwrap();
        assert_eq!(name, "Picture");
        let children = props["children"].as_list();
        assert_eq!(children.len(), 2);
        let (_, flow) = element(&children[0]).unwrap();
        assert_eq!(flow["steps"].as_list().len(), 4);
        // A callout alone is a picture too, and so is a flow alone.
        assert!(review("root = Picture([c])\nc = Callout(\"Mind the order.\")\n").is_empty());
        assert!(
            review("root = Picture([f])\nf = Flow([Step(\"In\"), Step(\"Out\")], \"The way through\")\n")
                .is_empty()
        );
    }

    #[test]
    fn the_library_holds_the_four_components() {
        let library = library();
        let names: Vec<&str> = library.names().collect();
        assert_eq!(names.len(), 4, "{names:?}");
        for name in ["Picture", "Flow", "Step", "Callout"] {
            assert!(names.contains(&name), "{name}");
        }
        assert_eq!(library.root, Some("Picture"));
        // What an earlier version drew is no component any more.
        for old in ["Card", "Stat", "BarChart", "Table"] {
            let issues = review(&format!("root = Picture([x])\nx = {old}([])\n"));
            assert!(
                issues.iter().any(|i| i.contains("unknown-component")),
                "{old}: {issues:?}"
            );
        }
    }

    /// The one issue a picture has, which must hold `expected`.
    fn refused(source: &str, expected: &str) {
        let issues = review(source);
        assert!(
            issues.iter().any(|issue| issue.contains(expected)),
            "expected `{expected}` for:\n{source}\ngot {issues:?}"
        );
    }

    #[test]
    fn a_picture_stays_within_its_limits() {
        let step = |n: usize| format!("Step(\"Step {n}\")");
        let flow = |count: usize| {
            let steps: Vec<String> = (1..=count).map(step).collect();
            format!("root = Picture([f])\nf = Flow([{}])\n", steps.join(", "))
        };
        assert!(review(&flow(MIN_STEPS)).is_empty());
        assert!(review(&flow(MAX_STEPS)).is_empty());
        refused(&flow(1), "the flow has 1 steps; it takes 2 to 7");
        refused(&flow(MAX_STEPS + 1), "the flow has 8 steps");

        let long = "x".repeat(MAX_STEP_TITLE + 1);
        refused(
            &format!("root = Picture([f])\nf = Flow([Step(\"{long}\"), Step(\"b\")])\n"),
            "the title of step 1 has 41 characters; at most 40",
        );
        let long = "x".repeat(MAX_STEP_TEXT + 1);
        refused(
            &format!("root = Picture([f])\nf = Flow([Step(\"a\"), Step(\"b\", \"{long}\")])\n"),
            "the text of step 2 has 161 characters",
        );
        refused(
            "root = Picture([f])\nf = Flow([Step(\" \"), Step(\"b\")])\n",
            "the title of step 1 is empty",
        );
        let long = "x".repeat(MAX_CALLOUT_TEXT + 1);
        refused(
            &format!("root = Picture([c])\nc = Callout(\"{long}\")\n"),
            "a callout's text has 241 characters",
        );
        refused(
            "root = Picture([c])\nc = Callout(\"Mind it.\", \"loud\")\n",
            "a callout's tone is \"loud\"; use info, warning, success, danger",
        );
        refused(
            "root = Picture([a, b])\na = Flow([Step(\"a\"), Step(\"b\")])\nb = Flow([Step(\"c\"), Step(\"d\")])\n",
            "the picture has 2 flows; at most one",
        );
        refused(
            "root = Picture([Callout(\"a\"), Callout(\"b\"), Callout(\"c\")])\n",
            "the picture has 3 callouts; at most 2",
        );
        refused("root = Picture([])\n", "the picture is empty");
        refused(
            "root = Picture([Step(\"alone\")])\n",
            "a Picture holds Flow and Callout only, not a Step",
        );
        refused(
            "root = Picture([f])\nf = Flow([Callout(\"a\"), Step(\"b\")])\n",
            "step 1 of the flow is not a Step",
        );
        refused(
            "root = Flow([Step(\"a\"), Step(\"b\")])\n",
            "`root` must be a Picture, not a Flow",
        );
        // Text that is no program is refused in the parser's own words.
        refused("just words\n", "parse-failed");
        refused(
            &format!(
                "root = Picture([c])\nc = Callout(\"ok\")\n// {}\n",
                "x".repeat(MAX_SOURCE)
            ),
            "at most 4000",
        );
    }

    #[test]
    fn a_picture_holds_plain_words() {
        refused(
            "root = Picture([c])\nc = Callout(\"Call `set` first.\")\n",
            "remove the backticks",
        );
        refused(
            "root = Picture([c])\nc = Callout(\"See [[Autostart Module]].\")\n",
            "holds no link",
        );
        refused(
            "root = Picture([c])\n# section: How it works\nc = Callout(\"a\")\n",
            "no line of a picture may start with `# section:`",
        );
    }

    #[test]
    fn the_limits_are_said_in_words() {
        let text = limits();
        for number in [
            MAX_PICTURES,
            MAX_CALLOUTS,
            MIN_STEPS,
            MAX_STEPS,
            MAX_STEP_TITLE,
            MAX_STEP_TEXT,
            MAX_CALLOUT_TEXT,
            MAX_SOURCE,
        ] {
            assert!(text.contains(&number.to_string()), "{number} in {text}");
        }
        for tone in TONES {
            assert!(text.contains(tone), "{tone}");
        }
    }

    #[test]
    fn a_file_keeps_the_pictures_of_a_note_by_section() {
        let pictures = vec![
            Picture {
                section: "What it does".into(),
                source: AUTOSTART.into(),
            },
            Picture {
                section: "How it works".into(),
                source: "root = Picture([c])\nc = Callout(\"Nothing else is stored.\")\n".into(),
            },
        ];
        let file = serialize_pictures(&pictures);
        assert!(file.starts_with("# section: What it does\nroot = Picture([flow, limit])\n"));
        assert!(file.contains("\n\n# section: How it works\nroot = Picture([c])\n"));
        assert_eq!(parse_pictures(&file), pictures);
        // Each picture of the file passes by itself, and the marks are
        // comments to the language.
        for picture in parse_pictures(&file) {
            assert!(review(&picture.source).is_empty(), "{}", picture.section);
        }
        // Text before the first mark, a mark with no name and a mark with
        // no picture hold nothing.
        let loose = "root = Picture([])\n# section: \nroot = Picture([])\n# section: Gotchas\n\n# section: Why\nroot = Picture([c])\nc = Callout(\"x\")\n";
        let kept = parse_pictures(loose);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].section, "Why");
        assert_eq!(serialize_pictures(&[]), "");
        assert!(parse_pictures("").is_empty());
    }

    #[test]
    fn quotes_text_for_a_program() {
        let source = format!(
            "root = Picture([c])\nc = Callout({})\n",
            quote("He said \"no\"\nand left\\")
        );
        let document = check(&source);
        assert!(document.is_valid(), "{:?}", document.errors);
        let (_, props) = element(document.root.as_ref().unwrap()).unwrap();
        let (_, callout) = element(&props["children"].as_list()[0]).unwrap();
        assert_eq!(text(callout, "text"), "He said \"no\"\nand left\\");
    }
}
