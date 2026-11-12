---
tags: [core, cw-llm, module]
---

# Writer Module

> Turns a structural draft into finished prose with an LLM.

## Source

- `crates/cw-llm/src/writer.rs` — module `cw_llm::writer`

## How it works

Main items:

- [[LlmWriter]] — struct
- `PROMPT_VERSION` — bump when the prompt changes in a way that should refresh notes.
- `MAX_SOURCE_CHARS` — Per-file and total caps on source text sent with one note.
- `MAX_TOTAL_SOURCE_CHARS` — const
- `extract_note` — remove a wrapping code fence and any text before the frontmatter.
- `SYSTEM` — const
- `kind_hint` — function
- `read_sources` — source text for the prompt, truncated at a line boundary.

## Depends on

- [[Generate Module]] — uses `WriterContext`
- [[NoteKind]] — uses `NoteKind`
- [[Planned]] — uses `Planned`

## Used by

- [[LlmWriter]] — from `name`, `rewrite`

## See also

- [[_index]]
- [[cw-llm]]
