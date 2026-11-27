---
tags: [core, cw-vault, module]
---

# Explanation Module

> Who writes which part of a note, and how the two parts are put together.

## What it does

A CodeWiki note has two authors. Rules write the facts from the code: where the thing is, its items, what it depends on. A coding agent writes the explanation: what the thing is for, how it works, what can go wrong. This module says which sections are the agent's, through [[NoteKind]], and puts the two parts together into one note.

It works in two directions. When an agent sends its text, the text goes into the note on disk. When the code changes and rules write the note again, the agent's text must survive into the new note. Both use the same function, `weave`, so the facts stay fresh and the explanation stays. Its callers are [[Explanations Module]] and [[Generate Module]].

## Source

- `crates/cw-vault/src/explanation.rs` — module `cw_vault::explanation`

## How it works

`weave` takes a base note, the agent's sections and the note's kind. The base depends on the caller:

- `submit` in [[Explanations Module]] weaves into the note on disk, after it has checked each section, the pictures and the lint rules. For a note its owner edited, the same text becomes a proposal.
- `weave_kept` in [[Generate Module]] weaves into the fresh draft that rules just wrote, when a sync finds that the code under an explained note changed. It also keeps sections such as `Gotchas` that rules never write.

Inside `weave`:

1. A kind with no agent sections, such as a glossary or decision note, gets the base back untouched.
2. Text under a heading the kind no longer has is "homeless". Its heading goes into `folded`, and the text is added to the first section the agent wrote, or else to the kind's first agent section.
3. `split` in [[Note Module]] cuts the base into its head (frontmatter, title, summary) and its `##` sections.
4. Each agent section replaces the body of the base section with the same heading. A section the agent did not send keeps the base's text.
5. In the `How it works` of a flow or operations note, the code blocks are facts: the diagram or the commands. The base's blocks stay first, and only the agent's prose follows them.
6. A section the base lacks goes before the first section that the kind's template places after it.
7. `join` puts the note together again. `written` lists the sections that now hold agent text, in template order.

`extract` goes the other way: it reads the agent's sections out of a note, without those fact blocks. `brief` uses it to show an agent its current text.

## Items

- `fn fences(body: &str) -> Vec<String>` — the fenced code blocks of a section's body, each with its fences, in their order.
- `fn prose(body: &str) -> String` — A section's body without its fenced code blocks.
- `pub fn extract(text: &str, kind: NoteKind, headings: &[String]) -> Vec<Section>` — the sections of the note `text` that an agent wrote: those under the `headings` the manifest records for it. In a section whose code blocks are facts, only the prose is the agent's. A heading that is no section of `kind` any more is still returned: the text is the agent's, and `weave` finds it another place.
- `pub struct Woven` — A note woven from a rule-written one and an agent's sections.
- `pub fn unlink_dead(text: &str, alive: impl Fn(&str) -> bool) -> String` — `text` with every wikilink that leads nowhere turned into plain words: the link's alias, or the name it pointed at. `alive` says whether a link's target leads to a note. A note an agent linked to may be gone when rules write the note again, and a dead link would fail the vault's check. Code is left as it is.
- `pub fn weave(base: &str, agent: &[Section], kind: NoteKind) -> Woven` — weave the note `base` with the sections an agent wrote. The head (frontmatter, title, summary) and every section that is not the agent's come from `base`, to the letter. An agent section the agent did not write keeps the text of `base`. In a section whose code blocks are facts, the blocks of `base` come first and the agent's prose after them. A section `base` lacks takes the place the kind's template gives it. A kind the agent owns no section of gets `base` back.

## Depends on

- [[NoteKind]] — uses `NoteKind`, `agent_sections`, `fact_fences`, `sections`
- [[Note Module]] — uses `join`, `split`
- [[Section]] — uses `Section`

## Used by

- [[cw-wiki]] — from `every_explainable_note_splits_and_joins_to_itself`, `explain`
- [[Explanations Module]] — from `brief`, `submit`
- [[Generate Module]] — from `weave_kept`
- [[Gone Module]] — from `after_removal`

## Gotchas

- Even with nothing new to put in, `weave` sends an explainable note through `split` and `join`. Blank lines are evened out and empty sections dropped, so a note laid out differently comes back changed. A test in [[cw-wiki]] checks that every explainable note splits and joins to itself.
- Which sections hold agent text comes from the headings the manifest (CodeWiki's record of each note) stores for it, not from the kind. A section of the agent's that still holds rule text is not read back, so rule text is never kept as an explanation.
- Text is never dropped for want of a place. When a section moves, [[Generate Module]] records a reason that keeps the note marked behind until an agent writes it again.
- A blank agent section is skipped and the base's text stays. Sending one cannot clear a section.
- In a section whose code blocks are facts, `weave` drops any code block the agent sent; `submit` refuses such a block before that. A block the base leaves open is closed at the end of its section.
- `unlink_dead` runs only on a sync. On a submit, a link to a missing note is refused instead.
- `unlink_dead` finds inline code by counting single backticks on a line. A link inside a span opened with two backticks counts as outside code and may be unlinked. Fenced blocks are never touched.

## See also

- [[_index]]
- [[cw-vault]]
