---
tags: [core, cw-vault, module]
---

# Lint Module

> Style linter for vault notes.

## What it does

Generated notes must pass with no errors before they are written. Adopted notes are linted in tolerant mode: structure rules become warnings because hand-written notes legitimately vary.

## Source

- `crates/cw-vault/src/lint.rs` — module `cw_vault::lint`

## How it works

1. `submit` in [[Explanations Module]] and `generate_with` in [[Generate Module]] call `lint`. `lint` calls `Note::parse`, then `extract_links` (both in [[Note Module]]), then `max_words`, then `lint_lines`. `lint_lines` calls `strip_inline_code`, then `has_inline_tag`, then `has_markdown_link`, then `has_line_number_ref`.
2. `clean_line` in [[Doctext Module]] and `label` in [[Explain Module]] call `is_line_ref`.
3. `generate_with` in [[Generate Module]] calls `link_resolution_rate`. `link_resolution_rate` calls `Note::parse` in [[Note Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub enum Severity` — public enum; used by `Issue`, `lint`, `lint_lines`.
- `pub struct Issue` — public struct; used by `lint`.
- `pub enum Mode` — public enum; used by `lint`.
- `pub const MIN_WORDS: usize` — public constant; used by `lint`.
- `pub const MAX_WORDS: usize` — the most words a note should hold, for every kind without a limit of its own.
- `pub const CONCEPT_MAX_WORDS: usize` — for a module, crate or type note, which explains its subject and lists every item with its whole comment.
- `pub const INTEGRATION_MAX_WORDS: usize` — for an integration note, which says what the package is and what each module takes from it.
- `pub fn max_words(kind: NoteKind) -> usize` — the most words a note of `kind` should hold. More is a warning.
- `const TOLERATED_EXTRA_SECTIONS: [&str; 7]` — sections hand-written vaults use in addition to the standard templates.
- `pub fn lint(text: &str, path: &str, kind: NoteKind, registry: &Registry, mode: Mode) -> Vec<Issue>` — lint one note. `path` is its vault-relative path.
- `lint_lines` — Line-level bans: callouts, checklists, inline tags, Markdown links, line numbers in code references.
- `fn strip_inline_code(line: &str) -> String` — private function, returns `String`; called by `lint_lines`.
- `fn has_inline_tag(prose: &str) -> bool` — `#word` at a word boundary, where `word` starts with a letter.
- `fn has_markdown_link(prose: &str) -> bool` — `[text](target)`, ignoring wikilinks.
- `fn has_line_number_ref(line: &str) -> bool` — A code span such as `` src/lib.rs:42 `` or `` run.py:25-30 ``.
- `pub fn is_line_ref(span: &str) -> bool` — whether the text of a code span reads as a file with a line number: src/lib.rs:42, run.py:25-30. An address such as 127.0.0.1:8080 reads the same, so text copied from a comment into a note must be checked with this before it is put in backticks.
- `link_resolution_rate` — share of wikilinks across `notes` that resolve, from 0.0 to 1.0.

## Depends on

- [[Note Module]] — uses `extract_links`, `parse`, `section`
- [[NoteKind]] — uses `NoteKind`, `sections`
- [[Registry]] — uses `Registry`, `resolve`

## Used by

- [[codewiki]] — from `note_style_skill_follows_the_lint`
- [[Doctext Module]] — from `clean_line`
- [[Explain Module]] — from `label`
- [[Explanations Module]] — from `submit`
- [[Generate Module]] — from `generate_with`

## See also

- [[_index]]
- [[cw-vault]]
