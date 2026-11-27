---
tags: [core, cw-vault, module]
---

# Note Module

> Parsing of a note's structure: frontmatter, title, summary, sections, links.

## What it does

Understands the shape of a wiki note. It reads a note's text into parts: frontmatter and tags, title, one-line summary, sections, links and word count. It also defines [[NoteKind]], which says which sections each kind of note has and which of them an agent writes. The linter, the explanation weaver and the health checks all rely on it.

## Source

- `crates/cw-vault/src/note.rs` — module `cw_vault::note`

## How it works

`Note::parse` walks the lines in four stages: before the title, expecting a summary, inside the summary, and the body. Frontmatter between `---` fences is read first and its tags go through `parse_tags`, which accepts `tags: [a, b]` or a block list. The `> ` lines under the title form the summary. Each `## ` heading starts a [[Section]]. Fenced code blocks are tracked, so their language is recorded and a heading inside them starts nothing. Links come from `extract_links`, and `count_words` counts prose only.

`split` and `join` are the exact way to cut a note into its head and sections and to put it together again. `join` is the one definition of a note's layout: a blank line, the heading, a blank line, the body. `weave` in [[Explanation Module]] uses both to replace the agent's sections and keep the rule-made ones.

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
- [[Gone Module]] — from `after_removal`

## Gotchas

- `join` leaves out a section with an empty body. A round trip through `split` and `join` keeps a note identical only if it has no empty sections.
- `extract_links` skips inline code spans, so a link in backticks is not a link. It accepts an alias and a heading part, and keeps only the target. A link with `!` before it is marked as an embed.
- Words in fenced code and frontmatter are not counted, so the length limits in [[Lint Module]] measure prose only.
- `Note::parse` and `split` both strip a byte order mark and read CRLF line endings the same as LF.
- `fact_fences` is true only for the `How it works` section of flow and operations notes. A diagram there is a rule-made fact, not agent text.
- `agent_sections` is empty for home, index, glossary and decision notes. No agent explains those.

## See also

- [[_index]]
- [[cw-vault]]
