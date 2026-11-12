---
tags: [core, cw-llm, module]
---

# Provider Module

> LLM providers: hosted APIs and locally installed agent CLIs.

## Source

- `crates/cw-llm/src/provider.rs` — module `cw_llm::provider`

## How it works

Main items:

- [[LlmProvider]] — trait
- [[AgentCli]] — an installed agent CLI run in non-interactive mode.
- [[Anthropic]] — struct
- `Ollama` — struct
- `OpenAiCompatible` — any server speaking the OpenAI chat-completions protocol.
- `Completion` — struct
- `DEFAULT_ANTHROPIC_MODEL` — default Anthropic model for prose generation.
- `from_env` — build the provider chosen by the environment.

## Depends on

- [[Error]] — uses `external`, `invalid`
- [[AgentCli]] — uses `claude`, `codex`
- [[cw-llm]] — uses `header`
- [[Error Module]] — uses `Result`
- [[Anthropic]] — uses `new`
- [[LlmProvider]] — uses `LlmProvider`

## Used by

- [[cw-llm]] — from `ollama_sends_a_non_streaming_chat_request`, `openai_compatible_sends_chat_completions_with_a_bearer_token`, `writer_from_env`
- [[Anthropic]] — from `complete`, `new`
- [[Illustrate Module]] — from `draw`
- [[Api Module]] — from `ask`
- [[AgentCli]] — from `complete`
- [[LlmProvider]] — from `complete`

## See also

- [[_index]]
- [[cw-llm]]
