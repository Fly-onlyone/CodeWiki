---
tags: [core, cw-llm]
---

# Ollama

> `Ollama` struct defined in `cw_llm::provider` with 4 methods.

## Source

- `crates/cw-llm/src/provider.rs` — definition of `Ollama`

## How it works

Declared as `pub struct Ollama`.

Public API:

- `fn name(&self) -> String`
- `fn probe(&self) -> Result<()>`
- `fn complete(&self, system: &str, user: &str) -> Result<Completion>`

## Depends on

- [[Provider Module]] — uses `Completion`, `PROBE_SYSTEM`, `PROBE_TIMEOUT`, `PROBE_USER`
- [[Error Module]] — uses `Result`
- [[LlmProvider]] — uses `LlmProvider`

## Used by

- [[cw-llm]] — from `a_failed_probe_carries_the_providers_explanation`, `a_probe_is_one_small_request_to_each_provider`, `ollama_sends_a_non_streaming_chat_request`
- [[Provider Module]] — from `from_config`

## See also

- [[_index]]
- [[Provider Module]]
