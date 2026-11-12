//! Asks an LLM to draw a note as an OpenUI Lang illustration.
//!
//! The model's output is never trusted: the generation pipeline checks it
//! against the component library and keeps the structural illustration when
//! the check fails.

use crate::provider::{Completion, LlmProvider};

const SYSTEM: &str = "You draw one illustration for a note in a codebase wiki. The illustration is shown beside the note in a desktop app and must help a reader grasp the note faster than its text: the flow, the parts and how they relate, or the numbers that matter.

Write the illustration in OpenUI Lang. A program is a list of assignments, one per line, `name = Component(arguments)`. Arguments are positional, in the order listed below. Strings use double quotes. Lists use square brackets and may hold names of other assignments or inline components. The assignment named `root` is the illustration and must be a `Card`.

Components (a `?` marks an optional argument):
- Card(children, title?) — the frame; root must be a Card.
- Stack(children, direction?, gap?) — direction \"row\" or \"column\"; gap \"s\", \"m\" or \"l\".
- Grid(children, columns?) — columns is a number.
- Tabs(items) with Tab(label, children).
- Divider()
- Header(title, subtitle?)
- TextContent(text, tone?) — tone \"default\", \"muted\", \"strong\", \"success\", \"warning\" or \"danger\".
- Stat(label, value, hint?) — value is a string.
- Badge(text, tone?)
- Steps(items) with Step(title, detail?) — an ordered sequence.
- Table(columns) with Col(label, values) — values is a list of strings, the same length in every column.
- BarChart(labels, series, title?) and LineChart(labels, series, title?) with Series(name, values) — values are numbers, one per label.
- PieChart(labels, values, title?)

Example:
root = Card([intro, flow], \"Request flow\")
intro = TextContent(\"A request passes three stages.\", \"muted\")
flow = Steps([s1, s2, s3])
s1 = Step(\"Parse\", \"`parse_request` validates the body\")
s2 = Step(\"Plan\", \"`plan` picks the handler\")
s3 = Step(\"Run\", \"`execute` writes the response\")

Rules:
- Use only the components above, only facts stated in the note, and at most 25 assignments.
- Prefer Steps for sequences, Table for comparisons, Stat for counts, charts only for real numbers from the note.
- Keep every string under 90 characters. No Markdown inside strings except backticks around identifiers.
- Output only the program. No code fence, no commentary.";

/// Remove a wrapping code fence and anything before the first assignment.
pub fn extract_program(response: &str) -> Option<String> {
    let lines: Vec<&str> = response
        .lines()
        .filter(|line| !line.trim_start().starts_with("```"))
        .collect();
    let first = lines.iter().position(|line| {
        line.split_once('=').is_some_and(|(name, _)| {
            let name = name.trim();
            !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
        })
    })?;
    let program = lines[first..].join("\n");
    let program = program.trim();
    (!program.is_empty()).then(|| format!("{program}\n"))
}

/// Ask the provider for an illustration of `note_text`. `structural` is the
/// illustration built from facts alone, offered as a starting point.
pub fn draw(
    provider: &dyn LlmProvider,
    note_text: &str,
    structural: Option<&str>,
) -> cw_core::Result<(Option<String>, Completion)> {
    let starting_point = match structural {
        Some(source) => format!(
            "<structural_illustration>\n{source}</structural_illustration>\n\nThe structural illustration above was built from counts alone. Keep what helps, and improve on it with what the note explains.\n\n"
        ),
        None => String::new(),
    };
    let user = format!("<note>\n{note_text}</note>\n\n{starting_point}Return the illustration.");
    let completion = provider.complete(SYSTEM, &user)?;
    Ok((extract_program(&completion.text), completion))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Canned(&'static str);

    impl LlmProvider for Canned {
        fn name(&self) -> String {
            "canned".into()
        }
        fn complete(&self, system: &str, user: &str) -> cw_core::Result<Completion> {
            assert!(system.contains("OpenUI Lang"));
            assert!(user.contains("<note>"));
            Ok(Completion {
                text: self.0.to_string(),
                input_tokens: 10,
                output_tokens: 5,
            })
        }
    }

    #[test]
    fn the_prompt_example_passes_the_library_check() {
        let start = SYSTEM.find("root = Card").unwrap();
        let end = SYSTEM.find("\n\nRules:").unwrap();
        let document = cw_illustrate::check(&SYSTEM[start..end]);
        assert!(document.is_valid(), "{:?}", document.errors);
    }

    #[test]
    fn strips_fences_and_leading_chatter() {
        let program = extract_program(
            "Here is the illustration:\n```openui\nroot = Card([t])\nt = TextContent(\"hi\")\n```\n",
        )
        .unwrap();
        assert_eq!(program, "root = Card([t])\nt = TextContent(\"hi\")\n");
        assert!(cw_illustrate::check(&program).is_valid());
        assert_eq!(extract_program("I cannot draw this."), None);
        assert_eq!(extract_program(""), None);
    }

    #[test]
    fn draw_offers_the_structural_illustration() {
        struct Echo;
        impl LlmProvider for Echo {
            fn name(&self) -> String {
                "echo".into()
            }
            fn complete(&self, _: &str, user: &str) -> cw_core::Result<Completion> {
                assert!(user.contains("<structural_illustration>\nroot = Card([])"));
                Ok(Completion {
                    text: "root = Card([])".into(),
                    input_tokens: 1,
                    output_tokens: 1,
                })
            }
        }
        let (program, _) = draw(&Echo, "# N\n", Some("root = Card([])\n")).unwrap();
        assert_eq!(program.as_deref(), Some("root = Card([])\n"));
        let (program, completion) = draw(&Canned("no program here"), "# N\n", None).unwrap();
        assert_eq!(program, None);
        assert_eq!(completion.output_tokens, 5);
    }
}
