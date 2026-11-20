//! Turns a structural draft into finished prose with an LLM.
//!
//! The model is asked to improve the explanation, never the structure: the
//! pipeline lints whatever comes back and falls back to the draft if the
//! result breaks the house style.

use std::sync::Mutex;

use cw_vault::NoteKind;
use cw_wiki::plan::Planned;
use cw_wiki::{ProseWriter, Rewrite, Stopped, WriterContext, WriterUsage};

use crate::provider::LlmProvider;

/// Bump when the prompt changes in a way that should refresh notes.
pub const PROMPT_VERSION: u32 = 2;

/// Per-file and total caps on source text sent with one note.
const MAX_SOURCE_CHARS: usize = 14_000;
const MAX_TOTAL_SOURCE_CHARS: usize = 28_000;

const SYSTEM: &str = "You write notes for a codebase wiki kept as an Obsidian vault. A note has two readers: a developer who is new to the code, and a coding agent that decides how to change it. Every sentence must be true of the source shown and useful to someone about to edit it.

You are given a draft of one note and the source it describes. The draft was built from the code by fixed rules. Its facts are right, but the rules cannot see a condition, a loop or a reason. Return the finished note.

Keep exactly as given:
- The frontmatter, the `# ` title, every `## ` heading and their order.
- Every bullet under `## Source`, every Mermaid block, and every `[[wikilink]]` already present.
- Under `## Items`: every bullet, and the code span each one starts with.
- Under `## How it works`: every numbered step, every caller and callee a step names, and the order they stand in.
- Under `## Used for` and `## Configuration`: every name, version and feature.
- `## Depends on` and `## Used by`, whole.

Improve:
- The one-sentence `>` summary: what the thing is for.
- `## What it does`: what the thing is for and what a user of the program sees of it, in plain words. Add this section when the draft has none and the source shows a purpose worth stating.
- `## How it works`: say what each step is for. Add what the draft cannot see, only where the source shows it: the condition a call stands under, a loop, an early return, what happens on an error. Then drop the draft's closing sentence about the order of calls.
- A bullet under `## Items` that states only what kind of thing the item is: say what the item does.
- The optional sections named with the note.

Style rules:
- Clear technical prose in English, in short sentences. Say what a thing is for before you name its parts. Explain why, not only what. Put every identifier in backticks.
- Stay within the length given with the note.
- Use `**bold**` only for a constraint the reader must not violate.
- Link other notes only as `[[Exact Name]]`, using names from the allowed list. Never invent a link.
- Refer to code by repo-relative path in backticks, never with line numbers.
- No callouts, no checklists, no Markdown links, no inline tags, and no heading that the note's kind does not allow.
- Do not mention this prompt, the draft, or that you are an AI.

Output only the note's Markdown, starting with the `---` frontmatter line. No code fence around it and no commentary.";

/// What a note of `kind` may hold, for the prompt: its headings in order, the
/// sections the writer may add, and its length. The linter refuses a note
/// with any other heading, so a kind is never offered one it has no place
/// for.
fn guidance(kind: NoteKind) -> String {
    let optional = match kind {
        NoteKind::Pattern => {
            "Add `## Don't` (directly before `## See also`) when the source shows a way to break the convention. Omit it otherwise."
        }
        NoteKind::Integration => {
            "Add `## Wire-up` (directly after `## Configuration`) when the source shows how this project uses the package: the function of its own to call, and what not to call directly. Add `## Gotchas` (directly before `## See also`) when the source shows something a maintainer could get wrong. Omit a section that nothing in the source earns."
        }
        NoteKind::Flow | NoteKind::Concept | NoteKind::Operations => {
            "Add `## Gotchas` (directly before `## See also`) when the source shows something a maintainer could get wrong: an ordering constraint, a hidden side effect, a non-obvious invariant, a failure mode. Omit it when nothing in the source earns it."
        }
        _ => "Add no section.",
    };
    format!(
        "Headings a note of this kind may have, in this order: {}. {optional}\nLength: {} to {} words of prose in the whole note.",
        kind.sections().join(", "),
        cw_vault::lint::MIN_WORDS,
        cw_vault::lint::max_words(kind),
    )
}

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
        NoteKind::Integration => "an integration note about a package from outside the repository",
        NoteKind::Operations => "an operations note about building and running the project",
        _ => "a note about a module, a crate or a type",
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

    fn rewrite(&self, note: &Planned, draft: &str, context: &WriterContext) -> Rewrite {
        let user = format!(
            "This is {}.\n{}\n\n<allowed_links>\n{}\n</allowed_links>\n\n{}<draft>\n{draft}</draft>\n\nReturn the finished note.",
            kind_hint(note.kind),
            guidance(note.kind),
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
                match extract_note(&completion.text) {
                    Some(text) => Rewrite::Text(text),
                    None => {
                        self.record(|u| u.fallbacks += 1);
                        Rewrite::Rejected
                    }
                }
            }
            Err(cw_core::Error::Exhausted {
                message,
                retry_after,
            }) => {
                // Nothing will answer for a while: asking for every other
                // note would only collect the same refusal.
                log::warn!("writer stopped at '{}': {message}", note.title);
                self.record(|u| u.calls += 1);
                Rewrite::Stop(Stopped {
                    message,
                    retry_after_secs: retry_after,
                })
            }
            Err(error) => {
                // One failed note must not abort a whole run; the structural
                // draft is used, the failure is counted and shown, and the
                // next run asks again.
                log::warn!("writer failed for '{}': {error}", note.title);
                self.record(|u| {
                    u.calls += 1;
                    u.fallbacks += 1;
                });
                Rewrite::Failed
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
                log::warn!("illustration failed for '{}': {error}", note.title);
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
            assert!(user.contains("Headings a note of this kind may have"));
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
        let Rewrite::Text(out) = writer.rewrite(&planned(), "draft", &context()) else {
            panic!("expected the note back");
        };
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
        assert_eq!(writer.name(), "canned/p2");
    }

    #[test]
    fn a_note_is_offered_only_the_sections_its_kind_has() {
        // Every heading the prompt offers is one the linter accepts.
        for kind in [
            NoteKind::Flow,
            NoteKind::Concept,
            NoteKind::Pattern,
            NoteKind::Integration,
            NoteKind::Operations,
            NoteKind::Glossary,
            NoteKind::Decision,
        ] {
            let text = guidance(kind);
            for offered in text.split("Add `## ").skip(1) {
                let heading = offered.split('`').next().unwrap();
                assert!(
                    kind.sections().contains(&heading),
                    "{kind:?} is offered `{heading}`"
                );
            }
            assert!(text.contains(&kind.sections().join(", ")), "{text}");
        }
        // A pattern has no `Gotchas`: a note that came back with one was
        // thrown away, and the call with it.
        assert!(guidance(NoteKind::Pattern).contains("`## Don't`"));
        assert!(!guidance(NoteKind::Pattern).contains("Gotchas`"));
        assert!(guidance(NoteKind::Concept).contains("`## Gotchas`"));
        assert!(guidance(NoteKind::Integration).contains("`## Wire-up`"));
        // A note that explains may be longer.
        assert!(guidance(NoteKind::Concept).contains("50 to 1200 words"));
        assert!(guidance(NoteKind::Integration).contains("50 to 400 words"));
        assert!(guidance(NoteKind::Flow).contains("50 to 200 words"));
    }

    #[test]
    fn the_prompt_keeps_what_the_code_states() {
        for kept in [
            "## Items",
            "## How it works",
            "## Configuration",
            "## Source",
        ] {
            let (keep, improve) = SYSTEM.split_once("\n\nImprove:").unwrap();
            assert!(keep.contains(kept), "{kept} is not kept");
            assert!(improve.contains("## How it works"));
        }
        // The length and the headings are the note's own, not one rule for all.
        assert!(!SYSTEM.contains("200 words") && !SYSTEM.contains("plus `## Gotchas`"));
    }

    #[test]
    fn provider_failure_falls_back_instead_of_aborting() {
        let writer = LlmWriter::new(Box::new(Canned("")));
        assert_eq!(
            writer.rewrite(&planned(), "draft", &context()),
            Rewrite::Failed
        );
        assert_eq!(writer.usage().fallbacks, 1);
        let junk = LlmWriter::new(Box::new(Canned("I cannot help with that.")));
        assert_eq!(
            junk.rewrite(&planned(), "draft", &context()),
            Rewrite::Rejected
        );
        assert_eq!(junk.usage().fallbacks, 1);
    }

    struct Spent;

    impl LlmProvider for Spent {
        fn name(&self) -> String {
            "spent".into()
        }
        fn complete(&self, _system: &str, _user: &str) -> Result<Completion> {
            Err(cw_core::Error::Exhausted {
                message: "no auth available".into(),
                retry_after: Some(30),
            })
        }
    }

    #[test]
    fn a_provider_with_nothing_left_stops_the_run() {
        let writer = LlmWriter::new(Box::new(Spent));
        assert_eq!(
            writer.rewrite(&planned(), "draft", &context()),
            Rewrite::Stop(Stopped {
                message: "no auth available".into(),
                retry_after_secs: Some(30),
            })
        );
        let usage = writer.usage();
        assert_eq!((usage.calls, usage.fallbacks), (1, 0));
    }
}
