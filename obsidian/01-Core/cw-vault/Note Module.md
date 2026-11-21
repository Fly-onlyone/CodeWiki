---
tags: [core, cw-vault, module]
---

# Note Module

> Parsing of a note's structure: frontmatter, title, summary, sections, links.

## Source

- `crates/cw-vault/src/note.rs` — module `cw_vault::note`

## How it works

1. `link_resolution_rate` and `lint` in [[Lint Module]], and 8 more, call `Note::parse`. `Note::parse` calls `parse_tags`, then `push_line`, then `extract_links`, then `count_words`.
2. `extract` and `weave` in [[Explanation Module]], and 4 more, call `split`.
3. `weave` in [[Explanation Module]] and `Doc::finish` in [[Render Module]] call `join`.
4. `lint` in [[Lint Module]] calls `extract_links`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- [[NoteKind]] — what a note documents.
- [[Section]] — struct
- `pub struct WikiLink` — public struct; used by `Note`, `extract_links`.
- `pub struct Note` — public struct; used by `adopted_kind`, `cited_sources`, `status_of`.
- `pub fn split(text: &str) -> (String, Vec<Section>)` — A note's head and its `## ` sections, in their order. The head is all that stands before the first section: frontmatter, title, summary and any prose under them. A `## ` line inside a fenced code block starts no section. `join` puts the two together again.
- `pub fn join(head: &str, sections: &[Section]) -> String` — A note from its head and its sections: the one definition of a note's layout. Each section is a blank line, its heading, a blank line and its body. A section with no body is left out.
- `fn push_line(current: &mut Option<Section>, preamble: &mut String, line: &str)` — private function; called by `Note::parse`.
- `fn parse_tags(front: &[String]) -> Vec<String>` — read `tags: [a, b]` or a YAML block list under `tags:`.
- `pub fn extract_links(text: &str) -> Vec<WikiLink>` — find `[[target]]`, `[[target|alias]]`, `[[target#heading]]` and `![[embed]]`. Text inside inline code spans is skipped.
- `fn count_words(prose: &str) -> usize` — private function, returns `usize`; called by `Note::parse`.

## Depends on

- [[Section]] — uses `Section`

## Used by

- [[Generate Module]] — from `adopt_existing`, `adopted_kind`, `cited_sources`, `keep_placed_pictures`
- [[Decisions Module]] — from `build`, `list`, `status_of`, `why`
- [[cw-wiki]] — from `a_submission_is_woven_into_the_note_and_recorded`, `a_submission_with_faults_writes_nothing_and_names_each`, `every_explainable_note_splits_and_joins_to_itself`
- [[Explanation Module]] — from `extract`, `weave`
- [[Lint Module]] — from `link_resolution_rate`, `lint`
- [[Explanations Module]] — from `brief`, `submit`
- [[Agentdocs Module]] — from `render`
- [[Health Module]] — from `doc_drift`

## See also

- [[_index]]
- [[cw-vault]]
