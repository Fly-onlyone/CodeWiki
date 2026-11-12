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
            Self::Flow | Self::Concept | Self::Operations => &[
                "Source",
                "How it works",
                "Depends on",
                "Used by",
                "Gotchas",
                "See also",
            ],
            Self::Pattern => &["When to apply", "The pattern", "Why", "Don't", "See also"],
            Self::Integration => &[
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
    fn detects_embeds() {
        let links = extract_links("See ![[Diagram]] and [[Plain]].");
        assert!(links[0].embed);
        assert!(!links[1].embed);
    }
}
