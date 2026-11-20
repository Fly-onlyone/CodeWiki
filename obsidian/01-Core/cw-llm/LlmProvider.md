---
tags: [core, cw-llm]
---

# LlmProvider

> `LlmProvider` trait defined in `cw_llm::provider` with 3 methods.

## Source

- `crates/cw-llm/src/provider.rs` — definition of `LlmProvider`

## How it works

What the functions of this type call:

1. `probe` calls `complete`.

## Items

Declared as `pub trait LlmProvider: Send + Sync`.

Required and provided methods:

- `fn name(&self) -> String` — identifier recorded with generated notes, for example `anthropic:claude-opus-5-5`.
- `fn complete(&self, system: &str, user: &str) -> Result<Completion>` — public method, returns `Result<Completion>`; called by `LlmProvider::probe`.
- `fn probe(&self) -> Result<()>` — check that the provider answers: one very small request. An error carries the provider's own explanation, such as a refused key or an unknown model.

Implemented by [[CliProxy]], [[AgentCli]], [[Anthropic]], `Logged`, [[Ollama]], and [[OpenAiCompatible]].

## Depends on

- [[Provider Module]] — uses `Completion`, `PROBE_SYSTEM`, `PROBE_USER`
- [[Error Module]] — uses `Result`

## Used by

- [[Provider Module]] — from `Logged`, `from_config`, `from_env`, `from_settings`
- [[LlmWriter]] — from `LlmWriter`, `new`
- [[Answer Module]] — from `answer`
- [[Illustrate Module]] — from `draw`
- [[CliProxy]] — from `CliProxy`
- [[AgentCli]] — from `AgentCli`
- [[Anthropic]] — from `Anthropic`
- [[Ollama]] — from `Ollama`

## See also

- [[_index]]
- [[Provider Module]]
- [[Implementing LlmProvider Pattern]]
