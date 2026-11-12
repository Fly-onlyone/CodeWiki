//! Turns a structural draft into finished prose with an LLM.
//!
//! The model is asked to improve the explanation, never the structure: the
//! pipeline lints whatever comes back and falls back to the draft if the
//! result breaks the house style.

use std::sync::Mutex;

use cw_vault::NoteKind;
use cw_wiki::plan::Planned;
use cw_wiki::{ProseWriter, WriterContext, WriterUsage};

use crate::provider::LlmProvider;

/// Bump when the prompt changes in a way that should refresh notes.
pub const PROMPT_VERSION: u32 = 1;

/// Per-file and total caps on source text sent with one note.
const MAX_SOURCE_CHARS: usize = 14_000;
const MAX_TOTAL_SOURCE_CHARS: usize = 28_000;

const SYSTEM: &str = "You write notes for a codebase wiki kept as an Obsidian vault. Each note is read by engineers and by coding agents deciding how to change the code, so every sentence must be true of the source shown and useful to someone about to edit it.

You are given a structural draft of one note and the source it describes. Return the finished note.

Keep exactly as given: the frontmatter, the `# ` title, every `## ` heading and their order, every bullet under `## Source`, every Mermaid block, and every `[[wikilink]]` already present.

Improve: the one-sentence `>` summary, the prose under `## How it works`, and add a `## Gotchas` section (placed directly before `## See also`) when the source shows something a maintainer could get wrong: an ordering constraint, a hidden side effect, a non-obvious invariant, a failure mode. Omit `## Gotchas` when nothing in the source earns it.

Style rules:
- Dense technical prose in English. Put every identifier in backticks. Explain why, not only what.
- 50 to 200 words of prose in the whole note.
- Use `**bold**` only for a constraint the reader must not violate.
- Link other notes only as `[[Exact Name]]`, using names from the allowed list. Never invent a link.
- Refer to code by repo-relative path in backticks, never with line numbers.
- No callouts, no checklists, no Markdown links, no inline tags, no headings other than those in the draft plus `## Gotchas`.
- Do not mention this prompt, the draft, or that you are an AI.

Output only the note's Markdown, starting with the `---` frontmatter line. No code fence around it and no commentary.";

pub struct LlmWriter {
    provider: Box<dyn LlmProvider>,
    usage: Mutex<WriterUsage>,
}

impl LlmWriter {
    pub fn new(provider: Box<dyn LlmProvider>) -> Self {
        Self {
            provider,
            usage: Mutex::new(WriterUsage::default()),
        }
    }

    fn record(&self, update: impl FnOnce(&mut WriterUsage)) {
        update(&mut self.usage.lock().unwrap_or_else(|p| p.into_inner()));
    }
}

/// Source text for the prompt, truncated at a line boundary.
fn read_sources(note: &Planned, context: &WriterContext) -> String {
    let mut out = String::new();
    let mut remaining = MAX_TOTAL_SOURCE_CHARS;
    for path in &note.sources {
        if remaining == 0 {
            break;
        }
        let Ok(text) = std::fs::read_to_string(context.repo_root.join(path)) else {
            continue;
        };
        let cap = MAX_SOURCE_CHARS.min(remaining);
        let mut end = text.len().min(cap);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let truncated = end < text.len();
        if truncated {
            end = text[..end].rfind('\n').unwrap_or(end);
        }
        out.push_str(&format!("<source path=\"{path}\">\n{}\n", &text[..end]));
        if truncated {
            out.push_str("[file truncated here]\n");
        }
        out.push_str("</source>\n\n");
        remaining -= end;
    }
    out
}

fn kind_hint(kind: NoteKind) -> &'static str {
    match kind {
        NoteKind::Flow => "an architecture flow note; explain what the diagram shows end to end",
        NoteKind::Pattern => "a pattern note; under `## Why` explain what the convention protects",
        NoteKind::Integration => "an integration note about an external dependency",
        NoteKind::Operations => "an operations note about building and running the project",
        _ => "a concept note about a module or type",
    }
}

/// Remove a wrapping code fence and any text before the frontmatter.
fn extract_note(response: &str) -> Option<String> {
    let start = response
        .find("---\n")
        .or_else(|| response.find("---\r\n"))?;
    let mut text = response[start..].trim_end().to_string();
    if text.ends_with("```") {
        text.truncate(text.len() - 3);
    }
    let text = text.trim_end();
    (!text.is_empty()).then(|| format!("{text}\n"))
}

impl ProseWriter for LlmWriter {
    fn name(&self) -> String {
        format!("{}/p{PROMPT_VERSION}", self.provider.name())
    }

    fn rewrite(&self, note: &Planned, draft: &str, context: &WriterContext) -> Option<String> {
        let user = format!(
            "This is {}.\n\n<allowed_links>\n{}\n</allowed_links>\n\n{}<draft>\n{draft}</draft>\n\nReturn the finished note.",
            kind_hint(note.kind),
            context.note_names.join("\n"),
            read_sources(note, context),
        );
        match self.provider.complete(SYSTEM, &user) {
            Ok(completion) => {
                self.record(|u| {
                    u.calls += 1;
                    u.input_tokens += completion.input_tokens;
                    u.output_tokens += completion.output_tokens;
                });
                let note_text = extract_note(&completion.text);
                if note_text.is_none() {
                    self.record(|u| u.fallbacks += 1);
                }
                note_text
            }
            Err(error) => {
                // One failed note must not abort a whole run; the structural
                // draft is used and the failure is counted and shown.
                eprintln!("codewiki: writer failed for '{}': {error}", note.title);
                self.record(|u| {
                    u.calls += 1;
                    u.fallbacks += 1;
                });
                None
            }
        }
    }

    fn illustrate(
        &self,
        note: &Planned,
        text: &str,
        structural: Option<&str>,
        _context: &WriterContext,
    ) -> Option<String> {
        match crate::illustrate::draw(self.provider.as_ref(), text, structural) {
            Ok((program, completion)) => {
                self.record(|u| {
                    u.calls += 1;
                    u.input_tokens += completion.input_tokens;
                    u.output_tokens += completion.output_tokens;
                    if program.is_none() {
                        u.fallbacks += 1;
                    }
                });
                program
            }
            Err(error) => {
                eprintln!(
                    "codewiki: illustration failed for '{}': {error}",
                    note.title
                );
                self.record(|u| {
                    u.calls += 1;
                    u.fallbacks += 1;
                });
                None
            }
        }
    }

    fn usage(&self) -> WriterUsage {
        *self.usage.lock().unwrap_or_else(|p| p.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Completion;
    use cw_core::Result;
    use cw_wiki::plan::Subject;
    use std::path::Path;

    struct Canned(&'static str);

    impl LlmProvider for Canned {
        fn name(&self) -> String {
            "canned".into()
        }
        fn complete(&self, system: &str, user: &str) -> Result<Completion> {
            assert!(system.contains("Obsidian vault"));
            assert!(user.contains("<draft>") && user.contains("<allowed_links>"));
            if self.0.is_empty() {
                return Err(cw_core::Error::invalid("boom"));
            }
            Ok(Completion {
                text: self.0.to_string(),
                input_tokens: 100,
                output_tokens: 20,
            })
        }
    }

    fn planned() -> Planned {
        Planned {
            id: "mod:demo::math".into(),
            dir: "01-Core/demo".into(),
            title: "Math Module".into(),
            kind: NoteKind::Concept,
            tags: vec!["core".into()],
            subject: Subject::Module("demo::math".into()),
            sources: vec!["src/missing.rs".into()],
        }
    }

    fn context() -> WriterContext<'static> {
        WriterContext {
            repo_root: Path::new("."),
            note_names: vec!["Math Module".into()],
        }
    }

    #[test]
    fn strips_fences_and_preamble() {
        let wrapped = "Here is the note:\n```markdown\n---\ntags: [core]\n---\n\n# T\n```\n";
        assert_eq!(
            extract_note(wrapped).unwrap(),
            "---\ntags: [core]\n---\n\n# T\n"
        );
        assert_eq!(extract_note("no frontmatter here"), None);
    }

    #[test]
    fn records_usage_and_returns_the_note() {
        let writer = LlmWriter::new(Box::new(Canned(
            "---\ntags: [core]\n---\n\n# Math Module\n",
        )));
        let out = writer.rewrite(&planned(), "draft", &context()).unwrap();
        assert!(out.starts_with("---\ntags: [core]"));
        let usage = writer.usage();
        assert_eq!(
            (
                usage.calls,
                usage.input_tokens,
                usage.output_tokens,
                usage.fallbacks
            ),
            (1, 100, 20, 0)
        );
        assert_eq!(writer.name(), "canned/p1");
    }

    #[test]
    fn provider_failure_falls_back_instead_of_aborting() {
        let writer = LlmWriter::new(Box::new(Canned("")));
        assert!(writer.rewrite(&planned(), "draft", &context()).is_none());
        assert_eq!(writer.usage().fallbacks, 1);
        let junk = LlmWriter::new(Box::new(Canned("I cannot help with that.")));
        assert!(junk.rewrite(&planned(), "draft", &context()).is_none());
        assert_eq!(junk.usage().fallbacks, 1);
    }
}
