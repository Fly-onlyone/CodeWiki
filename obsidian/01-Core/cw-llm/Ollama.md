---
tags: [core, cw-llm]
---

# Ollama

> `Ollama` struct defined in `cw_llm::provider` with 4 methods.

## Source

- `crates/cw-llm/src/provider.rs` — definition of `Ollama`

## How it works

What the functions of this type call:

1. `probe` calls `send`. `send` calls `post_json` in [[Provider Module]].
2. `complete` calls `send`.

## Items

Declared as `pub struct Ollama`.

Members:

- `fn send(&self, system: &str, user: &str, timeout: Duration) -> Result<Value>` — private method, returns `Result<Value>`; called by `Ollama::complete`, `Ollama::probe`.
- `fn name(&self) -> String` — public method, returns `String`.
- `fn probe(&self) -> Result<()>` — public method, returns `Result<()>`.
- `fn complete(&self, system: &str, user: &str) -> Result<Completion>` — public method, returns `Result<Completion>`.

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
