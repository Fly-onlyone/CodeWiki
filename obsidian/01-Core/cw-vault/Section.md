---
tags: [core, cw-vault]
---

# Section

> `Section` struct defined in `cw_vault::note`.

## What it does

One `## ` section of a note: its heading text and its body. It is the unit that CodeWiki and agents trade. An agent's explanation arrives as sections, and the weaver replaces matching sections of a note without touching the rest.

## Source

- `crates/cw-vault/src/note.rs` — definition of `Section`

## How it works

`Note::parse` in [[Note Module]] builds a list of sections while it reads lines, and trims each body. The `split` function does the same job with fewer rules and returns the head of the note plus its sections. `join` writes them back, leaving out a section whose body is empty.

## Items

Declared as `pub struct Section`.

## Used by

- [[Note Module]] — from `Note`, `join`, `parse`, `push_line`
- [[Explanation Module]] — from `extract`, `weave`
- [[Render Module]] — from `Doc`, `section`
- [[cw-wiki]] — from `explain`
- [[Explanations Module]] — from `submit`

## Gotchas

- The heading is stored without the `## ` marker, and the body without the heading line.
- A `## ` line inside a fenced code block is not a heading, so it never starts a section.
- Bodies are trimmed, so blank lines at the start or end of a section do not survive a round trip.

## See also

- [[_index]]
- [[Note Module]]
