---
tags: [core, cw-llm, module]
---

# Illustrate Module

> Asks an LLM to draw a note as an OpenUI Lang illustration.

## What it does

The model's output is never trusted: the generation pipeline checks it against the component library and keeps the structural illustration when the check fails.

## Source

- `crates/cw-llm/src/illustrate.rs` — module `cw_llm::illustrate`

## How it works

1. `LlmWriter::illustrate` in [[LlmWriter]] calls `draw`. `draw` calls `extract_program`.

## Items

- `const SYSTEM: &str` — private constant; used by `draw`.
- `pub fn extract_program(response: &str) -> Option<String>` — remove a wrapping code fence and anything before the first assignment.
- `draw` — ask the provider for an illustration of `note_text`. `structural` is the illustration built from facts alone, offered as a starting point.

## Depends on

- [[Error Module]] — uses `Result`
- [[Provider Module]] — uses `Completion`
- [[LlmProvider]] — uses `LlmProvider`

## Used by

- [[LlmWriter]] — from `illustrate`

## See also

- [[_index]]
- [[cw-llm]]
