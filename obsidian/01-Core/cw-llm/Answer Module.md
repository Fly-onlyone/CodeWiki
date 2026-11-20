---
tags: [core, cw-llm, module]
---

# Answer Module

> Answers a question about the codebase with an LLM, grounded in the vault.

## What it does

Retrieval is the same full-text search the keyless answer uses. The model sees the best-matching notes and symbols and must answer from them alone.

## Source

- `crates/cw-llm/src/answer.rs` — module `cw_llm::answer`

## How it works

1. `ask` in [[Api Module]] calls `answer`. `answer` calls `answer` in [[Query Module]], then `context`. `context` calls `clip`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const MAX_NOTES: usize` — most notes and characters of note text sent with one question.
- `fn clip(text: &str, cap: usize) -> &str` — private function, returns `&str`; called by `context`.
- `fn context(root: &Path, index: &Index, base: &Answer) -> Result<String>` — the material shown to the model for `answer`'s sources.
- `answer` — answer `question` with the provider. Falls back to the keyless answer when nothing in the vault matches or the provider fails, so a question always gets the best answer available.

And 2 more private items with no comment.

## Depends on

- [[Query Module]] — uses `Answer`, `answer`
- [[cw-core]] — uses `VAULT_DIR`
- [[Error Module]] — uses `Result`
- [[Index]] — uses `Index`
- [[LlmProvider]] — uses `LlmProvider`

## Used by

- [[Api Module]] — from `ask`

## See also

- [[_index]]
- [[cw-llm]]
