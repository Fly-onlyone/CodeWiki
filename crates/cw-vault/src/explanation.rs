//! Who writes which part of a note, and how the two parts are put together.
//!
//! Rules write a note from the code: where the thing is, its items, what it
//! depends on. A coding agent then explains it: what it is for, how it
//! works, what can go wrong. Each section of a note belongs to one of the
//! two ([`NoteKind::agent_sections`]). A note is always woven from both:
//! the rule-written note supplies its own sections, and the agent's text
//! takes the place of the sections that are the agent's.
//!
//! The same weave serves both ways. When the agent submits, the base is the
//! note on disk. When the code changes and rules write the note anew, the
//! base is that new note and the agent's sections are taken from the old
//! one: the facts are fresh and the explanation stays.

use crate::note::{self, NoteKind, Section};

/// The fenced code blocks of a section's body, each with its fences, in
/// their order.
fn fences(body: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut open: Option<Vec<&str>> = None;
    for line in body.lines() {
        let is_fence = line.trim_start().starts_with("```");
        match open.take() {
            Some(mut block) => {
                block.push(line);
                if is_fence {
                    blocks.push(block.join("\n"));
                } else {
                    open = Some(block);
                }
            }
            None if is_fence => open = Some(vec![line]),
            None => {}
        }
    }
    // A block the text never closes still ends with the section.
    if let Some(mut block) = open {
        block.push("```");
        blocks.push(block.join("\n"));
    }
    blocks
}

/// A section's body without its fenced code blocks.
fn prose(body: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut in_fence = false;
    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        } else if !in_fence {
            out.push(line);
        }
    }
    // Where a block stood, two blank lines may meet.
    let mut text = out.join("\n");
    while text.contains("\n\n\n") {
        text = text.replace("\n\n\n", "\n\n");
    }
    text.trim().to_string()
}

/// The agent's sections of the note `text`, of the headings in `headings`
/// that the note has and that are the agent's for a note of `kind`. In a
/// section whose code blocks are facts, only the prose is the agent's.
pub fn extract(text: &str, kind: NoteKind, headings: &[String]) -> Vec<Section> {
    let (_, sections) = note::split(text);
    sections
        .into_iter()
        .filter(|s| headings.contains(&s.heading) && kind.agent_sections().contains(&&*s.heading))
        .map(|s| Section {
            body: if kind.fact_fences(&s.heading) {
                prose(&s.body)
            } else {
                s.body
            },
            heading: s.heading,
        })
        .filter(|s| !s.body.is_empty())
        .collect()
}

/// A note woven from a rule-written one and an agent's sections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Woven {
    pub text: String,
    /// Headings of agent sections that have no place in a note of this
    /// kind any more. Their text was put into a section that has one.
    pub folded: Vec<String>,
}

/// Weave the note `base` with the sections an agent wrote. The head
/// (frontmatter, title, summary) and every section that is not the agent's
/// come from `base`, to the letter. An agent section the agent did not
/// write keeps the text of `base`. In a section whose code blocks are
/// facts, the blocks of `base` come first and the agent's prose after them.
/// A section `base` lacks takes the place the kind's template gives it.
///
/// A kind the agent owns no section of gets `base` back.
pub fn weave(base: &str, agent: &[Section], kind: NoteKind) -> Woven {
    let owned = kind.agent_sections();
    if owned.is_empty() {
        return Woven {
            text: base.to_string(),
            folded: Vec::new(),
        };
    }
    // What the agent wrote for each of its sections; text for a heading
    // that is no section of this kind any more waits to be folded in.
    let mut written: Vec<(&str, String)> = Vec::new();
    let mut folded = Vec::new();
    let mut homeless: Vec<String> = Vec::new();
    for section in agent {
        let body = section.body.trim();
        if body.is_empty() {
            continue;
        }
        match owned.iter().find(|heading| **heading == section.heading) {
            Some(heading) => written.push((*heading, body.to_string())),
            None => {
                folded.push(section.heading.clone());
                homeless.push(body.to_string());
            }
        }
    }
    if !homeless.is_empty() {
        // The first section the agent wrote takes it in, or the kind's
        // first: an explanation is kept sooner than a tidy layout.
        let home = written.first().map_or(owned[0], |(heading, _)| *heading);
        let extra = homeless.join("\n\n");
        match written.iter_mut().find(|(heading, _)| *heading == home) {
            Some((_, body)) => {
                body.push_str("\n\n");
                body.push_str(&extra);
            }
            None => written.push((home, extra)),
        }
    }

    let (head, mut sections) = note::split(base);
    let template = kind.sections();
    let place = |heading: &str| template.iter().position(|h| *h == heading);
    for (heading, text) in written {
        // The agent's text, with the facts of the section it goes into.
        let with_facts = |rule_body: &str| {
            if !kind.fact_fences(heading) {
                return text.clone();
            }
            let mut parts = fences(rule_body);
            parts.push(prose(&text));
            parts.retain(|part| !part.is_empty());
            parts.join("\n\n")
        };
        match sections.iter_mut().find(|s| s.heading == heading) {
            Some(section) => section.body = with_facts(&section.body),
            None => {
                let own = place(heading).unwrap_or(usize::MAX);
                // Before the first section the template puts after it. A
                // section the template does not know stays where it is.
                let at = sections
                    .iter()
                    .position(|s| place(&s.heading).is_some_and(|p| p > own))
                    .unwrap_or(sections.len());
                sections.insert(
                    at,
                    Section {
                        heading: heading.to_string(),
                        body: with_facts(""),
                    },
                );
            }
        }
    }
    Woven {
        text: note::join(&head, &sections),
        folded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(heading: &str, body: &str) -> Section {
        Section {
            heading: heading.into(),
            body: body.into(),
        }
    }

    const MODULE: &str = "---\ntags: [core, module]\n---\n\n# Autostart Module\n\n> Starting the app at sign-in.\n\n## What it does\n\nWindows runs what the Run key says.\n\n## Source\n\n- `src/autostart.rs` — module `app::autostart`\n\n## How it works\n\n1. `run_action` calls `set`.\n\n## Items\n\n- `pub fn set(on: bool)` — turn it on or off.\n\n## Used by\n\n- [[app]] — from `run_action`\n\n## See also\n\n- [[_index]]\n";

    #[test]
    fn the_agents_sections_take_their_places_and_the_facts_stay() {
        let agent = [
            section(
                "How it works",
                "`set(true)` writes the value; `set(false)` removes it.",
            ),
            section("Gotchas", "Only an installed copy offers this."),
        ];
        let woven = weave(MODULE, &agent, NoteKind::Concept);
        assert!(woven.folded.is_empty());
        let text = woven.text;
        // The head and the facts are the rule-written note's, to the letter.
        assert!(text.starts_with(
            "---\ntags: [core, module]\n---\n\n# Autostart Module\n\n> Starting the app at sign-in.\n\n## What it does\n\nWindows runs what the Run key says.\n\n## Source\n"
        ));
        assert!(text.contains("\n## Items\n\n- `pub fn set(on: bool)` — turn it on or off.\n"));
        assert!(text.contains(
            "\n## How it works\n\n`set(true)` writes the value; `set(false)` removes it.\n"
        ));
        assert!(!text.contains("`run_action` calls `set`."));
        // A section rules never write stands where the template has it.
        assert!(text.contains(
            "\n## Used by\n\n- [[app]] — from `run_action`\n\n## Gotchas\n\nOnly an installed copy offers this.\n\n## See also\n"
        ));
        // Nothing of the agent's: the note as rules wrote it.
        assert_eq!(weave(MODULE, &[], NoteKind::Concept).text, MODULE);

        // Read back, the agent's sections are what it wrote.
        let headings = vec!["How it works".to_string(), "Gotchas".to_string()];
        assert_eq!(extract(&text, NoteKind::Concept, &headings), agent);
        // A heading it did not write is not its text, and a fact never is.
        let all = vec![
            "What it does".to_string(),
            "Items".to_string(),
            "Gotchas".to_string(),
        ];
        let read = extract(&text, NoteKind::Concept, &all);
        let found: Vec<&str> = read.iter().map(|s| s.heading.as_str()).collect();
        assert_eq!(found, ["What it does", "Gotchas"]);
    }

    #[test]
    fn the_facts_change_under_an_explanation_that_stays() {
        let agent = [section("How it works", "It writes one registry value.")];
        let explained = weave(MODULE, &agent, NoteKind::Concept).text;
        // The code changes: rules write new facts, and a new walk-through.
        let fresh = MODULE
            .replace(
                "- `pub fn set(on: bool)` — turn it on or off.",
                "- `pub fn set(on: bool) -> bool` — turn it on or off; whether it changed.",
            )
            .replace(
                "1. `run_action` calls `set`.",
                "1. `run_action` calls `set`, then `log`.",
            );
        let kept = extract(&explained, NoteKind::Concept, &["How it works".to_string()]);
        let merged = weave(&fresh, &kept, NoteKind::Concept).text;
        assert!(merged.contains("`pub fn set(on: bool) -> bool`"));
        assert!(merged.contains("\n## How it works\n\nIt writes one registry value.\n"));
        assert!(!merged.contains("then `log`"));
    }

    const FLOW: &str = "---\ntags: [architecture, flow]\n---\n\n# app Startup Flow\n\n> Call sequence from `main`.\n\n## Source\n\n- `src/main.rs` — entry point\n\n## How it works\n\n```mermaid\nsequenceDiagram\n    p0->>p1: init()\n```\n\nThe sequence follows resolved calls outward from `main`.\n\n## See also\n\n- [[_index]]\n";

    #[test]
    fn a_diagram_is_a_fact_inside_a_section_of_the_agents() {
        let agent = [section(
            "How it works",
            "The app reads its settings before it opens a window.",
        )];
        let woven = weave(FLOW, &agent, NoteKind::Flow).text;
        // The rule-made block first, then the agent's prose in place of the
        // rule's sentence.
        assert!(
            woven.contains("## How it works\n\n```mermaid\nsequenceDiagram\n    p0->>p1: init()\n```\n\nThe app reads its settings before it opens a window.\n\n## See also"),
            "{woven}"
        );
        assert!(!woven.contains("follows resolved calls"));

        // A block the agent sent does not replace the rule's, and what is
        // read back is the prose alone.
        let sneaky = [section(
            "How it works",
            "Before.\n\n```mermaid\nflowchart LR\n  a --> b\n```\n\nAfter.",
        )];
        let woven = weave(FLOW, &sneaky, NoteKind::Flow).text;
        assert!(woven.contains("sequenceDiagram") && !woven.contains("flowchart LR"));
        let read = extract(&woven, NoteKind::Flow, &["How it works".to_string()]);
        assert_eq!(read, [section("How it works", "Before.\n\nAfter.")]);

        // The diagram changes with the code; the prose stays under it.
        let fresh = FLOW.replace("p0->>p1: init()", "p0->>p2: start()");
        let merged = weave(&fresh, &read, NoteKind::Flow).text;
        assert!(merged.contains("p0->>p2: start()") && merged.contains("Before.\n\nAfter."));
        assert!(!merged.contains("p0->>p1: init()"));
    }

    #[test]
    fn text_for_a_section_that_is_gone_is_kept_in_another() {
        // `Design notes` is no section of a module note.
        let agent = [
            section("Design notes", "The value is the whole setting."),
            section("Gotchas", "Windows has a second switch."),
        ];
        let woven = weave(MODULE, &agent, NoteKind::Concept);
        assert_eq!(woven.folded, ["Design notes"]);
        assert!(woven.text.contains(
            "## Gotchas\n\nWindows has a second switch.\n\nThe value is the whole setting.\n"
        ));
        assert!(!woven.text.contains("## Design notes"));
        // With nothing else written, the kind's first section takes it.
        let woven = weave(MODULE, &agent[..1], NoteKind::Concept);
        assert!(
            woven
                .text
                .contains("## What it does\n\nThe value is the whole setting.\n")
        );
        // A kind the agent owns nothing of keeps the note as it is.
        let woven = weave(MODULE, &agent, NoteKind::Glossary);
        assert_eq!(woven.text, MODULE);
        assert!(woven.folded.is_empty());
    }

    #[test]
    fn a_section_the_template_does_not_know_stays() {
        // A note edited by hand may hold a section of its owner's. The
        // woven text is what a proposal offers: it must not lose it.
        let edited = MODULE.replace(
            "## Used by",
            "## History\n\nWritten for the tray.\n\n## Used by",
        );
        let agent = [section("Gotchas", "Only an installed copy offers this.")];
        let woven = weave(&edited, &agent, NoteKind::Concept).text;
        assert!(woven.contains(
            "## History\n\nWritten for the tray.\n\n## Used by\n\n- [[app]] — from `run_action`\n\n## Gotchas\n\nOnly an installed copy offers this.\n\n## See also"
        ), "{woven}");
    }

    #[test]
    fn code_blocks_and_prose_are_told_apart() {
        let body = "One.\n\n```sh\ncargo test\n```\n\nTwo.\n\n```\nopen";
        assert_eq!(fences(body), ["```sh\ncargo test\n```", "```\nopen\n```"]);
        assert_eq!(prose(body), "One.\n\nTwo.");
        assert_eq!(prose("```\nonly code\n```"), "");
        assert!(fences("no code").is_empty());
    }
}
