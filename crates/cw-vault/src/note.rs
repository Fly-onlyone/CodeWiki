//! Parsing of a note's structure: frontmatter, title, summary, sections, links.

use serde::{Deserialize, Serialize};

/// What a note documents. Decides its section template and its zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteKind {
    /// `_HOME.md`, the root map.
    Home,
    /// `_index.md`, a folder's Map of Content.
    Index,
    /// Architecture flow, pipeline or lifecycle.
    Flow,
    /// Module, class or other code concept.
    Concept,
    Pattern,
    Integration,
    Operations,
    Glossary,
    /// A recorded decision and its reasons.
    Decision,
}

impl NoteKind {
    /// Section headings in required order. Empty sections are omitted by the
    /// writer, so every heading is optional but order is fixed.
    pub fn sections(self) -> &'static [&'static str] {
        match self {
            // What the thing is for comes before where it is and how it
            // runs: a reader new to the code starts at the purpose.
            Self::Concept => &[
                "What it does",
                "Source",
                "How it works",
                "Items",
                "Depends on",
                "Used by",
                "Gotchas",
                "See also",
            ],
            Self::Flow | Self::Operations => &[
                "Source",
                "How it works",
                "Depends on",
                "Used by",
                "Gotchas",
                "See also",
            ],
            Self::Pattern => &["When to apply", "The pattern", "Why", "Don't", "See also"],
            Self::Integration => &[
                "What it does",
                "Used for",
                "Configuration",
                "Wire-up",
                "Auth mode",
                "Gotchas",
                "See also",
            ],
            Self::Glossary => &["Definition", "See also"],
            Self::Decision => &["Status", "Context", "Decision", "Consequences", "See also"],
            Self::Home | Self::Index => &[],
        }
    }

    /// The sections of a note of this kind that a coding agent writes: what
    /// the thing is for, how it works, what can go wrong. Rules cannot know
    /// those. Every other section states a fact read from the code, and
    /// rules write it. Empty for a kind no agent explains.
    pub fn agent_sections(self) -> &'static [&'static str] {
        match self {
            Self::Concept => &["What it does", "How it works", "Gotchas"],
            Self::Flow | Self::Operations => &["How it works", "Gotchas"],
            Self::Pattern => &["Why", "Don't"],
            Self::Integration => &["What it does", "Wire-up", "Auth mode", "Gotchas"],
            Self::Home | Self::Index | Self::Glossary | Self::Decision => &[],
        }
    }

    /// Whether a coding agent may explain a note of this kind.
    pub fn explainable(self) -> bool {
        !self.agent_sections().is_empty()
    }

    /// Whether the fenced code blocks of the section `heading` are facts:
    /// the diagram of a flow, the commands of an operations note. Rules
    /// write them, inside a section whose prose is the agent's.
    pub fn fact_fences(self, heading: &str) -> bool {
        matches!(self, Self::Flow | Self::Operations) && heading == "How it works"
    }

    /// Kind implied by a note's file name, for map notes.
    pub fn from_file_name(name: &str) -> Option<Self> {
        match name {
            "_HOME.md" => Some(Self::Home),
            "_index.md" => Some(Self::Index),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub heading: String,
    /// Body text without the heading line, trimmed.
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    /// Target as written, before any `|alias` or `#heading`.
    pub target: String,
    pub alias: Option<String>,
    pub embed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Note {
    /// Raw frontmatter lines between the `---` fences, if present.
    pub frontmatter: Option<Vec<String>>,
    pub tags: Vec<String>,
    pub title: Option<String>,
    /// Text of the blockquote directly under the title.
    pub summary: Option<String>,
    /// Prose between the summary and the first `##` heading.
    pub preamble: String,
    pub sections: Vec<Section>,
    pub links: Vec<WikiLink>,
    /// Languages of fenced code blocks, in order (`mermaid`, `rust`, ...).
    pub code_languages: Vec<String>,
    /// Words of prose, excluding frontmatter and fenced code.
    pub word_count: usize,
}

impl Note {
    pub fn parse(text: &str) -> Self {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut lines: Vec<&str> = text.lines().collect();
        let mut note = Note::default();

        if lines.first().is_some_and(|l| l.trim_end() == "---")
            && let Some(end) = lines.iter().skip(1).position(|l| l.trim_end() == "---")
        {
            let front: Vec<String> = lines[1..=end].iter().map(|l| l.to_string()).collect();
            note.tags = parse_tags(&front);
            note.frontmatter = Some(front);
            lines.drain(..end + 2);
        }

        let mut in_code = false;
        let mut prose = String::new();
        let mut current: Option<Section> = None;
        let mut summary_lines: Vec<String> = Vec::new();
        // 0: before title, 1: expecting summary, 2: in summary, 3: body.
        let mut stage = 0u8;

        for line in lines {
            let trimmed = line.trim_end();
            if let Some(rest) = trimmed.trim_start().strip_prefix("```") {
                if !in_code {
                    note.code_languages.push(rest.trim().to_string());
                }
                in_code = !in_code;
                push_line(&mut current, &mut note.preamble, line);
                if stage == 2 {
                    stage = 3;
                }
                continue;
            }
            if in_code {
                push_line(&mut current, &mut note.preamble, line);
                continue;
            }
            if stage == 0 {
                if let Some(title) = trimmed.strip_prefix("# ") {
                    note.title = Some(title.trim().to_string());
                    stage = 1;
                    continue;
                }
                if trimmed.is_empty() {
                    continue;
                }
                // No H1: the note starts straight at its content.
                stage = 1;
            }
            if stage == 1 {
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed.starts_with('>') {
                    stage = 2;
                } else {
                    stage = 3;
                }
            }
            if stage == 2 {
                if let Some(q) = trimmed.strip_prefix('>') {
                    summary_lines.push(q.trim().to_string());
                    prose.push_str(q);
                    prose.push('\n');
                    continue;
                }
                stage = 3;
            }
            if let Some(heading) = trimmed.strip_prefix("## ") {
                if let Some(done) = current.take() {
                    note.sections.push(done);
                }
                current = Some(Section {
                    heading: heading.trim().to_string(),
                    body: String::new(),
                });
                continue;
            }
            prose.push_str(line);
            prose.push('\n');
            push_line(&mut current, &mut note.preamble, line);
        }
        if let Some(done) = current.take() {
            note.sections.push(done);
        }
        for s in &mut note.sections {
            s.body = s.body.trim().to_string();
        }
        note.preamble = note.preamble.trim().to_string();
        if !summary_lines.is_empty() {
            note.summary = Some(summary_lines.join(" ").trim().to_string());
        }
        note.links = extract_links(&prose);
        note.word_count = count_words(&prose);
        note
    }

    pub fn section(&self, heading: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.heading == heading)
    }
}

/// A note's head and its `## ` sections, in their order. The head is all
/// that stands before the first section: frontmatter, title, summary and
/// any prose under them. A `## ` line inside a fenced code block starts no
/// section. [`join`] puts the two together again.
pub fn split(text: &str) -> (String, Vec<Section>) {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut head: Vec<&str> = Vec::new();
    let mut sections: Vec<(String, Vec<&str>)> = Vec::new();
    let mut in_code = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
        } else if !in_code && let Some(heading) = line.trim_end().strip_prefix("## ") {
            sections.push((heading.trim().to_string(), Vec::new()));
            continue;
        }
        match sections.last_mut() {
            Some((_, body)) => body.push(line),
            None => head.push(line),
        }
    }
    let sections = sections
        .into_iter()
        .map(|(heading, body)| Section {
            heading,
            body: body.join("\n").trim().to_string(),
        })
        .collect();
    (head.join("\n").trim_end_matches('\n').to_string(), sections)
}

/// A note from its head and its sections: the one definition of a note's
/// layout. Each section is a blank line, its heading, a blank line and its
/// body. A section with no body is left out.
pub fn join(head: &str, sections: &[Section]) -> String {
    let mut out = head.trim_end_matches('\n').to_string();
    if !out.is_empty() {
        out.push('\n');
    }
    for section in sections {
        let body = section.body.trim();
        if !body.is_empty() {
            out.push_str(&format!("\n## {}\n\n{body}\n", section.heading));
        }
    }
    out
}

fn push_line(current: &mut Option<Section>, preamble: &mut String, line: &str) {
    let target = match current {
        Some(s) => &mut s.body,
        None => preamble,
    };
    target.push_str(line);
    target.push('\n');
}

/// Read `tags: [a, b]` or a YAML block list under `tags:`.
fn parse_tags(front: &[String]) -> Vec<String> {
    let mut tags = Vec::new();
    let mut in_block = false;
    for line in front {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("tags:") {
            let rest = rest.trim();
            if let Some(inner) = rest.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
                tags.extend(
                    inner
                        .split(',')
                        .map(|s| s.trim().trim_matches(['"', '\'']).to_string())
                        .filter(|s| !s.is_empty()),
                );
            } else if rest.is_empty() {
                in_block = true;
            }
            continue;
        }
        if in_block {
            match t.strip_prefix("- ") {
                Some(item) => tags.push(item.trim().trim_matches(['"', '\'']).to_string()),
                None => in_block = false,
            }
        }
    }
    tags
}

/// Find `[[target]]`, `[[target|alias]]`, `[[target#heading]]` and `![[embed]]`.
/// Text inside inline code spans is skipped.
pub fn extract_links(text: &str) -> Vec<WikiLink> {
    let mut links = Vec::new();
    for line in text.lines() {
        let bytes = line.as_bytes();
        let mut i = 0;
        let mut in_code = false;
        while i < bytes.len() {
            if bytes[i] == b'`' {
                in_code = !in_code;
                i += 1;
                continue;
            }
            if !in_code
                && bytes[i] == b'['
                && bytes.get(i + 1) == Some(&b'[')
                && let Some(len) = line[i + 2..].find("]]")
            {
                let inner = &line[i + 2..i + 2 + len];
                let (target, alias) = match inner.split_once('|') {
                    Some((t, a)) => (t, Some(a.trim().to_string())),
                    None => (inner, None),
                };
                let target = target.split('#').next().unwrap_or("").trim();
                if !target.is_empty() {
                    links.push(WikiLink {
                        target: target.to_string(),
                        alias,
                        embed: i > 0 && bytes[i - 1] == b'!',
                    });
                }
                i += len + 4;
                continue;
            }
            i += 1;
        }
    }
    links
}

fn count_words(prose: &str) -> usize {
    prose
        .split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_splits_into_its_sections_and_joins_again() {
        let text = "---\ntags: [core]\n---\n\n# Engine\n\n> Runs jobs.\n\n## Source\n\n- `src/engine.rs`\n\n## How it works\n\n```md\n## not a heading\n```\n\nIt runs.\n\n## See also\n\n- [[_index]]\n";
        let (head, sections) = split(text);
        assert_eq!(head, "---\ntags: [core]\n---\n\n# Engine\n\n> Runs jobs.");
        let headings: Vec<&str> = sections.iter().map(|s| s.heading.as_str()).collect();
        assert_eq!(headings, ["Source", "How it works", "See also"]);
        assert_eq!(sections[1].body, "```md\n## not a heading\n```\n\nIt runs.");
        assert_eq!(join(&head, &sections), text);
        // The same sections `parse` finds.
        assert_eq!(Note::parse(text).sections, sections);
        // Windows line ends and a byte order mark read the same.
        let windows = format!("{}{}", '\u{feff}', text.replace('\n', "\r\n"));
        assert_eq!(split(&windows), (head.clone(), sections.clone()));

        // A section with no body is left out, and a note may have none.
        let mut fewer = sections.clone();
        fewer[0].body.clear();
        assert!(!join(&head, &fewer).contains("## Source"));
        assert_eq!(join(&head, &[]), format!("{head}\n"));
        assert_eq!(split("just words"), ("just words".to_string(), vec![]));
    }

    #[test]
    fn each_kind_says_which_sections_an_agent_writes() {
        for kind in [
            NoteKind::Concept,
            NoteKind::Flow,
            NoteKind::Operations,
            NoteKind::Pattern,
            NoteKind::Integration,
        ] {
            assert!(kind.explainable(), "{kind:?}");
            // Every one of them is a section of the kind, and rules keep
            // at least one section, the links at the end among them.
            for heading in kind.agent_sections() {
                assert!(kind.sections().contains(heading), "{kind:?} {heading}");
            }
            assert!(!kind.agent_sections().contains(&"See also"));
            assert!(kind.agent_sections().len() < kind.sections().len());
        }
        for kind in [
            NoteKind::Home,
            NoteKind::Index,
            NoteKind::Glossary,
            NoteKind::Decision,
        ] {
            assert!(!kind.explainable(), "{kind:?}");
        }
        // Rules keep the items of a module and the settings of an
        // integration.
        assert!(!NoteKind::Concept.agent_sections().contains(&"Items"));
        assert!(
            !NoteKind::Integration
                .agent_sections()
                .contains(&"Configuration")
        );
        assert!(NoteKind::Flow.fact_fences("How it works"));
        assert!(NoteKind::Operations.fact_fences("How it works"));
        assert!(!NoteKind::Flow.fact_fences("Gotchas"));
        assert!(!NoteKind::Concept.fact_fences("How it works"));
    }

    const NOTE: &str = "---\ntags: [backend, handlers]\n---\n\n# HuntModeHandler\n\n> Time-targeted purchase handler — launches its own browser.\n\n## Source\n\n- `backend/handlers/HuntModeHandler.py` — primary implementation\n\n## How it works\n\nThree phases.\n\n```mermaid\nflowchart LR\n    A[[not a link]] --> B\n```\n\nPolls with [[Bot Entry Point]] and `[[ignored]]`.\n\n## See also\n\n- [[_index]]\n- [[01-Backend/_index|Backend]]\n- [[Hunt Mode Lifecycle#Phases]]\n";

    #[test]
    fn parses_structure() {
        let n = Note::parse(NOTE);
        assert_eq!(n.tags, ["backend", "handlers"]);
        assert_eq!(n.title.as_deref(), Some("HuntModeHandler"));
        assert_eq!(
            n.summary.as_deref(),
            Some("Time-targeted purchase handler — launches its own browser.")
        );
        let headings: Vec<&str> = n.sections.iter().map(|s| s.heading.as_str()).collect();
        assert_eq!(headings, ["Source", "How it works", "See also"]);
        assert!(
            n.section("How it works")
                .unwrap()
                .body
                .contains("flowchart LR")
        );
        assert_eq!(n.code_languages, ["mermaid"]);
    }

    #[test]
    fn extracts_links_outside_code() {
        let n = Note::parse(NOTE);
        let targets: Vec<&str> = n.links.iter().map(|l| l.target.as_str()).collect();
        assert_eq!(
            targets,
            [
                "Bot Entry Point",
                "_index",
                "01-Backend/_index",
                "Hunt Mode Lifecycle"
            ]
        );
        assert_eq!(n.links[2].alias.as_deref(), Some("Backend"));
    }

    #[test]
    fn counts_prose_words_only() {
        let n = Note::parse(
            "# T\n\n> One two.\n\n## S\n\nThree four — five.\n\n```rust\nlet ignored = 1;\n```\n",
        );
        assert_eq!(n.word_count, 5);
    }

    #[test]
    fn handles_multi_line_summary_and_missing_title() {
        let n = Note::parse(
            "---\ntags:\n  - moc\n  - home\n---\n> First line\n> second line.\n\nBody.\n",
        );
        assert_eq!(n.tags, ["moc", "home"]);
        assert_eq!(n.title, None);
        assert_eq!(n.summary.as_deref(), Some("First line second line."));
        assert_eq!(n.preamble, "Body.");
    }

    #[test]
    fn a_concept_and_an_integration_open_with_their_purpose() {
        for kind in [NoteKind::Concept, NoteKind::Integration] {
            assert_eq!(kind.sections()[0], "What it does", "{kind:?}");
        }
        let concept = NoteKind::Concept.sections();
        let at = |heading: &str| concept.iter().position(|h| *h == heading).unwrap();
        // The walk-through, then the reference list it names.
        assert!(at("Source") < at("How it works"));
        assert!(at("How it works") < at("Items"));
        assert!(at("Items") < at("Depends on"));
        // A flow's `How it works` is its diagram; it has no items.
        assert!(!NoteKind::Flow.sections().contains(&"Items"));
        assert_eq!(NoteKind::Flow.sections(), NoteKind::Operations.sections());
    }

    #[test]
    fn detects_embeds() {
        let links = extract_links("See ![[Diagram]] and [[Plain]].");
        assert!(links[0].embed);
        assert!(!links[1].embed);
    }
}
