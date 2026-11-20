---
tags: [core, cw-vault, module]
---

# Note Module

> Parsing of a note's structure: frontmatter, title, summary, sections, links.

## Source

- `crates/cw-vault/src/note.rs` — module `cw_vault::note`

## How it works

1. `lint` in [[Lint Module]] and `Note::parse` in [[Note]] call `extract_links`.
2. `Note::parse` in [[Note]] calls `push_line`.
3. `Note::parse` in [[Note]] calls `parse_tags`.
4. `Note::parse` in [[Note]] calls `count_words`.

## Items

- [[NoteKind]] — what a note documents.
- `pub struct Section` — public struct; used by `Note`, `Note::parse`, `Note::section`, `push_line`.
- `pub struct WikiLink` — public struct; used by `Note`, `extract_links`.
- [[Note]] — struct
- `fn push_line(current: &mut Option<Section>, preamble: &mut String, line: &str)` — private function; called by `Note::parse`.
- `fn parse_tags(front: &[String]) -> Vec<String>` — read `tags: [a, b]` or a YAML block list under `tags:`.
- `pub fn extract_links(text: &str) -> Vec<WikiLink>` — find `[[target]]`, `[[target|alias]]`, `[[target#heading]]` and `![[embed]]`. Text inside inline code spans is skipped.
- `fn count_words(prose: &str) -> usize` — private function, returns `usize`; called by `Note::parse`.

## Used by

- [[Note]] — from `Note`, `parse`, `section`
- [[Lint Module]] — from `lint`

## See also

- [[_index]]
- [[cw-vault]]
