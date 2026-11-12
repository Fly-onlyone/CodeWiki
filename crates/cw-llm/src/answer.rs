//! Answers a question about the codebase with an LLM, grounded in the vault.
//!
//! Retrieval is the same full-text search the keyless answer uses. The model
//! sees the best-matching notes and symbols and must answer from them alone.

use std::path::Path;

use cw_core::{Result, VAULT_DIR};
use cw_index::Index;
use cw_wiki::query::{self, Answer};

use crate::provider::LlmProvider;

/// Most notes and characters of note text sent with one question.
const MAX_NOTES: usize = 4;
const MAX_NOTE_CHARS: usize = 6_000;

const SYSTEM: &str = "You answer questions about one codebase for engineers and coding agents. You are given notes from the codebase's wiki and symbols from its index. Answer from that material only.

Rules:
- Lead with the answer. Then give the reasoning a reader needs to act on it.
- Put every identifier and path in backticks. Name the note a claim comes from as `[[Note Name]]`.
- When the material does not answer the question, say so plainly and name what to look at next. Never guess.
- At most 200 words. Plain Markdown, no headings.";

fn clip(text: &str, cap: usize) -> &str {
    if text.len() <= cap {
        return text;
    }
    let mut end = cap;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..text[..end].rfind('\n').unwrap_or(end)]
}

/// The material shown to the model for `answer`'s sources.
fn context(root: &Path, index: &Index, base: &Answer) -> Result<String> {
    let vault = root.join(VAULT_DIR);
    let mut out = String::new();
    for hit in base
        .sources
        .iter()
        .filter(|h| h.kind == "note")
        .take(MAX_NOTES)
    {
        if let Ok(text) = std::fs::read_to_string(vault.join(&hit.key)) {
            out.push_str(&format!(
                "<note name=\"{}\" path=\"{VAULT_DIR}/{}\">\n{}\n</note>\n\n",
                hit.title,
                hit.key,
                clip(&text, MAX_NOTE_CHARS)
            ));
        }
    }
    for hit in base.sources.iter().filter(|h| h.kind == "symbol") {
        if let Some(symbol) = index.symbol(&hit.key)? {
            out.push_str(&format!(
                "<symbol name=\"{}\" file=\"{}\">\n{}\n{}\n</symbol>\n\n",
                symbol.qualified, symbol.file, symbol.signature, symbol.doc
            ));
        }
    }
    Ok(out)
}

/// Answer `question` with the provider. Falls back to the keyless answer
/// when nothing in the vault matches or the provider fails, so a question
/// always gets the best answer available.
pub fn answer(
    provider: &dyn LlmProvider,
    root: &Path,
    index: &Index,
    question: &str,
) -> Result<Answer> {
    let base = query::answer(root, index, question)?;
    let material = context(root, index, &base)?;
    if material.is_empty() {
        return Ok(base);
    }
    let user = format!("{material}<question>\n{question}\n</question>");
    match provider.complete(SYSTEM, &user) {
        Ok(completion) if !completion.text.trim().is_empty() => Ok(Answer {
            answer: completion.text.trim().to_string(),
            ..base
        }),
        Ok(_) => Ok(base),
        Err(error) => {
            eprintln!("codewiki: answer fell back to note summaries: {error}");
            Ok(base)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Completion;
    use std::fs;

    struct Canned(Option<&'static str>);

    impl LlmProvider for Canned {
        fn name(&self) -> String {
            "canned".into()
        }
        fn complete(&self, system: &str, user: &str) -> Result<Completion> {
            assert!(system.contains("Answer from that material only"));
            assert!(user.contains("<note name=\"engine\""), "{user}");
            assert!(user.contains("<symbol name=\"engine::run\""), "{user}");
            assert!(user.ends_with("<question>\nhow does the engine run\n</question>"));
            match self.0 {
                Some(text) => Ok(Completion {
                    text: text.into(),
                    input_tokens: 1,
                    output_tokens: 1,
                }),
                None => Err(cw_core::Error::invalid("provider down")),
            }
        }
    }

    fn fixture() -> (tempfile::TempDir, Index) {
        let dir = tempfile::tempdir().unwrap();
        let write = |rel: &str, text: &str| {
            let path = dir.path().join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        };
        write(
            "Cargo.toml",
            "[package]\nname = \"engine\"\ndescription = \"Order matching engine\"\n",
        );
        write(
            "src/lib.rs",
            "//! Engine.\n/// Run the engine once.\npub fn run() {}\n",
        );
        let mut index = Index::open_in_memory().unwrap();
        let facts = cw_wiki::service::reindex(dir.path(), &mut index).unwrap();
        cw_wiki::generate(dir.path(), &facts, &cw_wiki::analyze(&facts), None, None).unwrap();
        cw_wiki::service::index_notes(dir.path(), &mut index).unwrap();
        (dir, index)
    }

    #[test]
    fn answers_from_notes_and_symbols() {
        let (dir, index) = fixture();
        let question = "how does the engine run";
        let reply = answer(
            &Canned(Some("  It runs once. See [[engine]].\n")),
            dir.path(),
            &index,
            question,
        )
        .unwrap();
        assert_eq!(reply.answer, "It runs once. See [[engine]].");
        assert!(reply.sources.iter().any(|h| h.kind == "note"));
    }

    #[test]
    fn falls_back_when_the_provider_fails_or_nothing_matches() {
        let (dir, index) = fixture();
        let keyless = query::answer(dir.path(), &index, "how does the engine run").unwrap();
        let reply = answer(&Canned(None), dir.path(), &index, "how does the engine run").unwrap();
        assert_eq!(reply.answer, keyless.answer);
        // No match: the provider is never called (it would fail its asserts).
        let reply = answer(&Canned(Some("x")), dir.path(), &index, "zzzzqqq").unwrap();
        assert!(reply.answer.contains("No note in the vault matches"));
    }

    #[test]
    fn clip_cuts_at_a_line_inside_the_cap() {
        assert_eq!(clip("ab\ncd\nef", 5), "ab");
        assert_eq!(clip("short", 50), "short");
        assert_eq!(clip("é\né", 2), "é");
    }
}
