---
tags: [core, cw-llm]
---

# LlmProvider

> `LlmProvider` trait defined in `cw_llm::provider` with 2 methods.

## Source

- `crates/cw-llm/src/provider.rs` — definition of `LlmProvider`

## How it works

Declared as `pub trait LlmProvider: Send + Sync`.

Required and provided methods:

- `fn name(&self) -> String` — identifier recorded with generated notes, for example `anthropic:claude-opus-5-5`.
- `fn complete(&self, system: &str, user: &str) -> Result<Completion>`

Implemented by [[AgentCli]], [[Anthropic]], `Ollama`, and `OpenAiCompatible`.

## Depends on

- [[Error Module]] — uses `Result`
- [[Provider Module]] — uses `Completion`

## Used by

- [[Provider Module]] — from `Ollama`, `OpenAiCompatible`, `from_env`
- [[LlmWriter]] — from `LlmWriter`, `new`
- [[Answer Module]] — from `answer`
- [[Illustrate Module]] — from `draw`
- [[AgentCli]] — from `AgentCli`
- [[Anthropic]] — from `Anthropic`

## See also

- [[_index]]
- [[Provider Module]]
- [[Implementing LlmProvider Pattern]]
