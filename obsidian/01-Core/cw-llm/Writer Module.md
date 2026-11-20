---
tags: [core, cw-llm, module]
---

# Writer Module

> Turns a structural draft into finished prose with an LLM.

## What it does

The model is asked to improve the explanation, never the structure: the pipeline lints whatever comes back and falls back to the draft if the result breaks the house style.

## Source

- `crates/cw-llm/src/writer.rs` — module `cw_llm::writer`

## How it works

1. `LlmWriter::rewrite` in [[LlmWriter]] calls `guidance`. `guidance` calls `max_words` in [[Lint Module]].
2. `LlmWriter::rewrite` in [[LlmWriter]] calls `read_sources`.
3. `LlmWriter::rewrite` in [[LlmWriter]] calls `kind_hint`.
4. `LlmWriter::rewrite` in [[LlmWriter]] calls `extract_note`.

## Items

- `pub const PROMPT_VERSION: u32` — bump when the prompt changes in a way that should refresh notes.
- `const MAX_SOURCE_CHARS: usize` — Per-file and total caps on source text sent with one note.
- `fn guidance(kind: NoteKind) -> String` — what a note of `kind` may hold, for the prompt: its headings in order, the sections the writer may add, and its length. The linter refuses a note with any other heading, so a kind is never offered one it has no place for.
- [[LlmWriter]] — struct
- `fn read_sources(note: &Planned, context: &WriterContext) -> String` — source text for the prompt, truncated at a line boundary.
- `fn kind_hint(kind: NoteKind) -> &'static str` — private function, returns `&'static str`; called by `LlmWriter::rewrite`.
- `fn extract_note(response: &str) -> Option<String>` — remove a wrapping code fence and any text before the frontmatter.

And 2 more private items with no comment.

## Depends on

- [[Lint Module]] — uses `MIN_WORDS`, `max_words`
- [[NoteKind]] — uses `NoteKind`, `sections`
- [[Generate Module]] — uses `WriterContext`
- [[Planned]] — uses `Planned`

## Used by

- [[LlmWriter]] — from `name`, `rewrite`

## See also

- [[_index]]
- [[cw-llm]]
