---
tags: [core, cw-vault, module]
---

# Lint Module

> Style linter for vault notes.

## What it does

The style checker for vault notes. It reads a note's text and returns a list of issues, each with a rule name, a severity and a message. Notes that CodeWiki writes are checked in strict mode and must have no errors before they are saved. Notes a person wrote or edited are checked in tolerant mode, where structure rules only warn.

## Source

- `crates/cw-vault/src/lint.rs` — module `cw_vault::lint`

## How it works

`lint` parses the text with `Note::parse` from [[Note Module]]. The severity of structure rules, called structural, is `Error` in strict mode and `Warning` in tolerant mode. It checks in this order:

1. Frontmatter holds only `tags`, and the tags are not empty.
2. A `# Title` and a one-line `>` summary exist. The home note may have its summary after an introduction.
3. Sections follow the template of the note's [[NoteKind]]: known names, right order, none empty. Tolerant mode accepts seven extra section names that hand-written vaults use.
4. A `See also` section exists and its first link is `_index` (or `_HOME` for top-level maps).
5. Length: fewer than 50 words, or more than the kind's maximum (`max_words`), is a warning.
6. Every link must resolve in the [[Registry]]; embeds are banned.
7. `lint_lines` scans line by line, skipping frontmatter and fenced code. It bans callouts, checklists, inline `#tags`, Markdown links and code spans that read as a file with a line number.

`link_resolution_rate` reports the share of links across many notes that resolve.

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

## Gotchas

- An unresolved link is always an `Error`, even in tolerant mode.
- `is_line_ref` also matches an address such as a host and port. Text copied from a comment must pass this check before it goes in backticks, or the note fails the lint.
- Inline code is stripped before the tag and link checks, so a code span can hold `#word` safely.
- Length limits differ: 200 words for most kinds, 1200 for module, crate and type notes, 400 for integrations. Exceeding them is only a warning.
- Mermaid diagrams outside a flow note give a warning in strict mode only.
- The note-writing skill follows these rules by hand. A test in the `codewiki` crate checks that the skill and the lint agree.

## See also

- [[_index]]
- [[cw-vault]]
