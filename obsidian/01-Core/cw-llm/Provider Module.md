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
- [[Anthropic]] — struct
- [[Ollama]] — struct
- [[OpenAiCompatible]] — any server speaking the OpenAI chat-completions protocol.
- [[AgentCli]] — an installed agent CLI run in non-interactive mode.
- `Completion` — struct
- `from_settings` — the provider chosen by the settings in effect: the environment over the settings file.
- `from_config` — build the provider that `lookup` describes.

## Depends on

- [[Settings Module (cw-core)]] — uses `ANTHROPIC_API_KEY`, `LLM_MODEL`, `LLM_PROVIDER`, `OLLAMA_HOST`
- [[Error]] — uses `Error`, `external`, `invalid`
- [[AgentCli]] — uses `claude`, `codex`
- [[Error Module]] — uses `Result`
- [[Key]] — uses `Key`
- [[Resolved]] — uses `Resolved`
- [[Anthropic]] — uses `new`
- [[LlmProvider]] — uses `LlmProvider`

## Used by

- [[Anthropic]] — from `complete`, `new`, `probe`, `send`
- [[Settings Module (cw-server)]] — from `checked`, `describe`, `test_llm`
- [[Ollama]] — from `complete`, `probe`, `send`
- [[OpenAiCompatible]] — from `complete`, `probe`, `send`
- [[LlmProvider]] — from `complete`, `probe`
- [[cw-llm]] — from `writer_from`
- [[Serve Module]] — from `writer_name`
- [[Illustrate Module]] — from `draw`

## See also

- [[_index]]
- [[cw-llm]]
