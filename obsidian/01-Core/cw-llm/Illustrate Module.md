---
tags: [core, cw-llm, module]
---

# Illustrate Module

> Asks an LLM to draw a note as an OpenUI Lang illustration.

## Source

- `crates/cw-llm/src/illustrate.rs` — module `cw_llm::illustrate`

## How it works

Main items:

- `extract_program` — remove a wrapping code fence and anything before the first assignment.
- `draw` — ask the provider for an illustration of `note_text`.
- `SYSTEM` — const

## Depends on

- [[Error Module]] — uses `Result`
- [[Provider Module]] — uses `Completion`
- [[LlmProvider]] — uses `LlmProvider`

## Used by

- [[LlmWriter]] — from `illustrate`

## See also

- [[_index]]
- [[cw-llm]]
