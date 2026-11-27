---
tags: [core, cw-vault]
---

# NoteKind

> What a note documents.

## What it does

Names the kind of thing a note documents: home, index, flow, concept, pattern, integration, operations, glossary or decision. The kind decides which sections the note has, in which order, and which of them an agent writes. The linter and the explanation tools both ask it.

## Source

- `crates/cw-vault/src/note.rs` — definition of `NoteKind`

## How it works

`sections` returns the heading list of the kind in required order. Every heading is optional, because the writer leaves empty ones out, but the order is fixed. A concept note opens with `What it does` and puts the walk-through before the item list.

`agent_sections` returns the part an agent writes: the purpose, the walk-through and what can go wrong. Rules write every other section from facts in the code. `explainable` is true when that list is not empty. `fact_fences` says that in a flow or operations note the code blocks of `How it works` are facts that rules write inside an agent's prose section. `from_file_name` maps `_HOME.md` and `_index.md` to their kinds; other file names have no implied kind.

## Items

Declared as `pub enum NoteKind`.

Members:

- `pub fn sections(self) -> &'static [&'static str]` — section headings in required order. Empty sections are omitted by the writer, so every heading is optional but order is fixed.
- `pub fn agent_sections(self) -> &'static [&'static str]` — the sections of a note of this kind that a coding agent writes: what the thing is for, how it works, what can go wrong. Rules cannot know those. Every other section states a fact read from the code, and rules write it. Empty for a kind no agent explains.
- `pub fn explainable(self) -> bool` — whether a coding agent may explain a note of this kind.
- `pub fn fact_fences(self, heading: &str) -> bool` — whether the fenced code blocks of the section `heading` are facts: the diagram of a flow, the commands of an operations note. Rules write them, inside a section whose prose is the agent's.
- `pub fn from_file_name(name: &str) -> Option<Self>` — kind implied by a note's file name, for map notes.

## Used by

- [[Explanations Module]] — from `Brief`, `Waiting`, `brief`, `review_section`
- [[cw-wiki]] — from `a_run_counts_the_notes_no_agent_has_explained`, `a_swift_axaml_repository_is_mapped_at_file_level`, `every_explainable_note_splits_and_joins_to_itself`, `the_line_about_waiting_notes_goes_when_none_waits`
- [[Generate Module]] — from `adopt_existing`, `adopted_kind`, `generate_with`, `weave_kept`
- [[codewiki]] — from `has_template`, `note_style_skill_follows_the_lint`, `write_notes_skill_follows_the_tools_and_the_limits`
- [[Lint Module]] — from `lint`, `lint_lines`, `max_words`
- [[Query Module]] — from `NoteInfo`, `agent_view`, `info`
- [[Explanation Module]] — from `extract`, `weave`
- [[Registry Module]] — from `RegistryEntry`

## Gotchas

- Home, index, glossary and decision notes have no agent sections. No agent explains them.
- `See also` is never an agent section, and every agent section is also in the kind's own section list. A test checks both.
- A flow has no `Items` section, and flow and operations notes share one section list.
- Pattern notes use `Why` and `Don't` as the agent's sections, and integrations use `What it does`, `Wire-up`, `Auth mode` and `Gotchas`.
- The serialized name is snake case, so a new kind changes `notes.jsonl`.

## See also

- [[_index]]
- [[Note Module]]
