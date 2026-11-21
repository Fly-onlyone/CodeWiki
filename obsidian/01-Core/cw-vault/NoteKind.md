---
tags: [core, cw-vault]
---

# NoteKind

> What a note documents.

## What it does

Decides its section template and its zone.

## Source

- `crates/cw-vault/src/note.rs` — definition of `NoteKind`

## How it works

1. `adopt_existing` and `adopted_kind` in [[Generate Module]] call `from_file_name`.

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
- [[cw-wiki]] — from `a_run_counts_the_notes_no_agent_has_explained`, `every_explainable_note_splits_and_joins_to_itself`, `the_line_about_waiting_notes_goes_when_none_waits`, `the_queue_lists_what_waits_and_counts_all_of_it`
- [[Generate Module]] — from `adopt_existing`, `adopted_kind`, `generate_with`, `weave_kept`
- [[codewiki]] — from `has_template`, `note_style_skill_follows_the_lint`, `write_notes_skill_follows_the_tools_and_the_limits`
- [[Lint Module]] — from `lint`, `lint_lines`, `max_words`
- [[Query Module]] — from `NoteInfo`, `agent_view`, `info`
- [[Explanation Module]] — from `extract`, `weave`
- [[Registry Module]] — from `RegistryEntry`

## See also

- [[_index]]
- [[Note Module]]
