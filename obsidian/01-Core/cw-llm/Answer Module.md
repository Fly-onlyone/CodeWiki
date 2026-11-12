---
tags: [core, cw-llm, module]
---

# Answer Module

> Answers a question about the codebase with an LLM, grounded in the vault.

## Source

- `crates/cw-llm/src/answer.rs` — module `cw_llm::answer`

## How it works

Main items:

- `answer` — answer `question` with the provider.
- `context` — the material shown to the model for `answer`'s sources.
- `MAX_NOTES` — most notes and characters of note text sent with one question.
- `MAX_NOTE_CHARS` — const
- `clip` — function
- `SYSTEM` — const

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
