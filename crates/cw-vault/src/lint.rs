//! Style linter for vault notes.
//!
//! Generated notes must pass with no errors before they are written. Adopted
//! notes are linted in tolerant mode: structure rules become warnings because
//! hand-written notes legitimately vary.

use serde::{Deserialize, Serialize};

use crate::note::{Note, NoteKind};
use crate::registry::Registry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// For notes CodeWiki writes.
    Strict,
    /// For notes a human wrote or edited.
    Tolerant,
}

pub const MIN_WORDS: usize = 50;
pub const MAX_WORDS: usize = 200;

/// Sections hand-written vaults use in addition to the standard templates.
const TOLERATED_EXTRA_SECTIONS: [&str; 7] = [
    "Key fields",
    "Endpoints",
    "Special features",
    "Overview",
    "Steps",
    "Notes",
    "Definition",
];

/// Lint one note. `path` is its vault-relative path.
pub fn lint(text: &str, path: &str, kind: NoteKind, registry: &Registry, mode: Mode) -> Vec<Issue> {
    let note = Note::parse(text);
    let mut issues = Vec::new();
    let structural = if mode == Mode::Strict {
        Severity::Error
    } else {
        Severity::Warning
    };
    let mut push = |rule: &'static str, severity: Severity, message: String| {
        issues.push(Issue {
            rule,
            severity,
            message,
        });
    };

    match &note.frontmatter {
        None => push("frontmatter", structural, "missing frontmatter".into()),
        Some(lines) => {
            let keys: Vec<&str> = lines
                .iter()
                .filter(|l| !l.starts_with([' ', '\t', '-']) && l.contains(':'))
                .map(|l| l.split(':').next().unwrap_or("").trim())
                .collect();
            if keys != ["tags"] {
                push(
                    "frontmatter",
                    structural,
                    format!(
                        "frontmatter must hold only `tags`, found: {}",
                        keys.join(", ")
                    ),
                );
            }
            if note.tags.is_empty() {
                push("frontmatter", structural, "`tags` is empty".into());
            }
        }
    }

    if note.title.is_none() {
        push("title", structural, "missing `# Title` heading".into());
    }
    // The home note opens with an introduction, then its summary blockquote.
    let home_summary = kind == NoteKind::Home && note.preamble.lines().any(|l| l.starts_with("> "));
    if note.summary.is_none() && !home_summary {
        push(
            "summary",
            structural,
            "missing one-sentence `>` summary under the title".into(),
        );
    }

    let template = kind.sections();
    if !template.is_empty() {
        let mut last_position = 0usize;
        for section in &note.sections {
            match template.iter().position(|h| *h == section.heading) {
                Some(position) => {
                    if position < last_position {
                        push(
                            "section-order",
                            structural,
                            format!("section `{}` is out of order", section.heading),
                        );
                    }
                    last_position = position;
                }
                None => {
                    let tolerated = mode == Mode::Tolerant
                        && TOLERATED_EXTRA_SECTIONS.contains(&section.heading.as_str());
                    if !tolerated {
                        push(
                            "section-unknown",
                            structural,
                            format!("section `{}` is not part of the template", section.heading),
                        );
                    }
                }
            }
            if section.body.is_empty() {
                push(
                    "section-empty",
                    structural,
                    format!("section `{}` is empty; drop it instead", section.heading),
                );
            }
        }
    }

    let expected_first = match kind {
        NoteKind::Home => None,
        // Zone maps link up to the home note; nested maps to their parent map.
        NoteKind::Index => Some(if path.matches('/').count() <= 1 {
            "_HOME"
        } else {
            "_index"
        }),
        _ => Some("_index"),
    };
    if let Some(expected) = expected_first {
        match note.section("See also") {
            None => push(
                "see-also",
                structural,
                "missing `## See also` section".into(),
            ),
            Some(section) => {
                let first = crate::note::extract_links(&section.body)
                    .into_iter()
                    .next()
                    .map(|l| l.target);
                let ok = first.as_deref().is_some_and(|t| {
                    t == expected
                        || (kind == NoteKind::Index && (t == "_HOME" || t.ends_with("_index")))
                });
                if !ok {
                    push(
                        "see-also",
                        structural,
                        format!("`See also` must start with `[[{expected}]]`"),
                    );
                }
            }
        }
    }

    if !matches!(kind, NoteKind::Home | NoteKind::Index) {
        if note.word_count < MIN_WORDS {
            push(
                "length",
                Severity::Warning,
                format!("{} words; aim for at least {MIN_WORDS}", note.word_count),
            );
        } else if note.word_count > MAX_WORDS {
            push(
                "length",
                Severity::Warning,
                format!("{} words; aim for at most {MAX_WORDS}", note.word_count),
            );
        }
    }

    for link in &note.links {
        if link.embed {
            push(
                "no-embeds",
                structural,
                format!("embed `![[{}]]` is not allowed", link.target),
            );
        }
        if registry.resolve(&link.target, path).is_none() {
            push(
                "link-unresolved",
                Severity::Error,
                format!("link `[[{}]]` does not resolve", link.target),
            );
        }
    }

    if mode == Mode::Strict
        && note.code_languages.iter().any(|l| l == "mermaid")
        && kind != NoteKind::Flow
    {
        push(
            "mermaid-placement",
            Severity::Warning,
            "Mermaid diagrams belong in architecture notes".into(),
        );
    }

    lint_lines(text, kind, structural, &mut push);
    issues
}

/// Line-level bans: callouts, checklists, inline tags, Markdown links, line
/// numbers in code references.
fn lint_lines(
    text: &str,
    kind: NoteKind,
    structural: Severity,
    push: &mut impl FnMut(&'static str, Severity, String),
) {
    let mut in_code = false;
    let mut in_frontmatter = false;
    for (number, line) in text.lines().enumerate() {
        let t = line.trim();
        if number == 0 && t == "---" {
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter {
            in_frontmatter = t != "---";
            continue;
        }
        if t.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        if t.starts_with("> [!") {
            push(
                "no-callouts",
                structural,
                format!("line {}: callouts are not used", number + 1),
            );
        }
        if t.starts_with("- [ ]") || t.starts_with("- [x]") || t.starts_with("- [X]") {
            push(
                "no-checklists",
                structural,
                format!("line {}: checklists are not used", number + 1),
            );
        }
        let prose = strip_inline_code(t);
        if has_inline_tag(&prose) {
            push(
                "no-inline-tags",
                structural,
                format!("line {}: tags belong in frontmatter", number + 1),
            );
        }
        if has_markdown_link(&prose) {
            push(
                "wikilinks-only",
                structural,
                format!(
                    "line {}: use `[[wikilinks]]`, not Markdown links",
                    number + 1
                ),
            );
        }
        if kind != NoteKind::Home && has_line_number_ref(t) {
            push(
                "no-line-numbers",
                structural,
                format!(
                    "line {}: code references must not carry line numbers",
                    number + 1
                ),
            );
        }
    }
}

fn strip_inline_code(line: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    for c in line.chars() {
        if c == '`' {
            in_code = !in_code;
        } else if !in_code {
            out.push(c);
        }
    }
    out
}

/// `#word` at a word boundary, where `word` starts with a letter.
fn has_inline_tag(prose: &str) -> bool {
    let chars: Vec<char> = prose.chars().collect();
    chars.iter().enumerate().any(|(i, &c)| {
        c == '#'
            && (i == 0 || chars[i - 1].is_whitespace())
            && chars.get(i + 1).is_some_and(|n| n.is_alphabetic())
    })
}

/// `[text](target)`, ignoring wikilinks.
fn has_markdown_link(prose: &str) -> bool {
    let bytes = prose.as_bytes();
    (0..bytes.len()).any(|i| {
        bytes[i] == b']'
            && bytes.get(i + 1) == Some(&b'(')
            && (i == 0 || bytes[i - 1] != b']')
            && prose[..i]
                .rfind('[')
                .is_some_and(|open| open == 0 || bytes[open - 1] != b'[')
    })
}

/// A code span such as `` `src/lib.rs:42` `` or `` `run.py:25-30` ``.
fn has_line_number_ref(line: &str) -> bool {
    line.split('`').skip(1).step_by(2).any(|span| {
        span.rsplit_once(':').is_some_and(|(path, tail)| {
            path.contains('.')
                && !path.contains(' ')
                && !tail.is_empty()
                && tail.chars().all(|c| c.is_ascii_digit() || c == '-')
                && tail.chars().next().is_some_and(|c| c.is_ascii_digit())
        })
    })
}

/// Share of wikilinks across `notes` that resolve, from 0.0 to 1.0.
pub fn link_resolution_rate<'a>(
    notes: impl IntoIterator<Item = (&'a str, &'a str)>,
    registry: &Registry,
) -> f64 {
    let (mut total, mut resolved) = (0usize, 0usize);
    for (path, text) in notes {
        for link in Note::parse(text).links {
            total += 1;
            if registry.resolve(&link.target, path).is_some() {
                resolved += 1;
            }
        }
    }
    if total == 0 {
        1.0
    } else {
        resolved as f64 / total as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::RegistryEntry;

    fn registry() -> Registry {
        let mut r = Registry::new();
        for (path, kind) in [
            ("_HOME.md", NoteKind::Home),
            ("01-Core/_index.md", NoteKind::Index),
            ("01-Core/Widget.md", NoteKind::Concept),
            ("01-Core/Gadget.md", NoteKind::Concept),
            ("04-Patterns/_index.md", NoteKind::Index),
        ] {
            r.insert(RegistryEntry {
                id: path.into(),
                path: path.into(),
                kind,
            })
            .unwrap();
        }
        r
    }

    fn good() -> String {
        let filler = "It keeps a small cache of parsed values and refreshes them when the source file changes. \
                      Callers treat it as read-only state and never mutate the cache directly, because \
                      a stale entry would silently produce wrong output for every later request.";
        format!(
            "---\ntags: [core]\n---\n\n# Widget\n\n> Holds parsed values for the renderer.\n\n## Source\n\n- `src/widget.rs` — implementation\n\n## How it works\n\n{filler}\n\n## Depends on\n\n- [[Gadget]] — supplies raw values\n\n## See also\n\n- [[_index]]\n"
        )
    }

    fn rules(issues: &[Issue]) -> Vec<&'static str> {
        issues.iter().map(|i| i.rule).collect()
    }

    #[test]
    fn accepts_a_well_formed_note() {
        let issues = lint(
            &good(),
            "01-Core/Widget.md",
            NoteKind::Concept,
            &registry(),
            Mode::Strict,
        );
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn flags_structure_problems() {
        let text = "# Widget\n\n## How it works\n\nShort.\n\n## Source\n\n- `src/widget.rs:42` — impl\n\n## Extras\n\n## See also\n\n- [[Gadget]]\n";
        let issues = lint(
            text,
            "01-Core/Widget.md",
            NoteKind::Concept,
            &registry(),
            Mode::Strict,
        );
        let r = rules(&issues);
        for expected in [
            "frontmatter",
            "summary",
            "section-order",
            "section-unknown",
            "section-empty",
            "see-also",
            "length",
            "no-line-numbers",
        ] {
            assert!(r.contains(&expected), "missing {expected} in {r:?}");
        }
    }

    #[test]
    fn flags_banned_constructs_but_not_inside_code() {
        let text = good()
            + "\n> [!note] Callout\n- [ ] todo\nSee #tag and [site](https://example.com) and ![[Gadget]] and [[Nope]].\n`#not_a_tag` and `[x](y)` are fine.\n```\n# heading in code\n- [ ] in code\n```\n";
        let issues = lint(
            &text,
            "01-Core/Widget.md",
            NoteKind::Concept,
            &registry(),
            Mode::Strict,
        );
        let r = rules(&issues);
        for expected in [
            "no-callouts",
            "no-checklists",
            "no-inline-tags",
            "wikilinks-only",
            "no-embeds",
            "link-unresolved",
        ] {
            assert_eq!(
                r.iter().filter(|x| **x == expected).count(),
                1,
                "{expected} in {r:?}"
            );
        }
    }

    #[test]
    fn tolerant_mode_downgrades_structure_but_not_broken_links() {
        let text = "# Widget\n\n## Key fields\n\n- a\n\n## See also\n\n- [[Nope]]\n";
        let issues = lint(
            text,
            "01-Core/Widget.md",
            NoteKind::Concept,
            &registry(),
            Mode::Tolerant,
        );
        assert!(!rules(&issues).contains(&"section-unknown"));
        let errors: Vec<&Issue> = issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].rule, "link-unresolved");
    }

    #[test]
    fn index_notes_link_up_first() {
        let text = "---\ntags: [moc, core]\n---\n\n# Core — Map of Content\n\n> Core types.\n\n## Notes\n\n- [[Widget]] — parsed values\n\n## See also\n\n- [[_HOME]]\n";
        let issues = lint(
            text,
            "01-Core/_index.md",
            NoteKind::Index,
            &registry(),
            Mode::Strict,
        );
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn computes_link_resolution_rate() {
        let r = registry();
        let notes = [
            ("01-Core/Widget.md", "[[Gadget]] [[_index]] [[_HOME]]"),
            ("01-Core/Gadget.md", "[[Missing]]"),
        ];
        assert_eq!(link_resolution_rate(notes, &r), 0.75);
        assert_eq!(link_resolution_rate([], &r), 1.0);
    }
}
