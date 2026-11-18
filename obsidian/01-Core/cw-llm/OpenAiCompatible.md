---
tags: [core, cw-llm]
---

# OpenAiCompatible

> Any server speaking the OpenAI chat-completions protocol.

## Source

- `crates/cw-llm/src/provider.rs` — definition of `OpenAiCompatible`

## How it works

Declared as `pub struct OpenAiCompatible`.

Public API:

- `fn probe(&self) -> Result<()>` — no output limit is sent: servers of this protocol do not agree on the name of that field.
- `fn name(&self) -> String`
- `fn complete(&self, system: &str, user: &str) -> Result<Completion>`

## Depends on

- [[Provider Module]] — uses `Completion`, `PROBE_SYSTEM`, `PROBE_TIMEOUT`, `PROBE_USER`
- [[Error Module]] — uses `Result`
- [[LlmProvider]] — uses `LlmProvider`

## Used by

- [[cw-llm]] — from `a_failed_probe_carries_the_providers_explanation`, `a_probe_is_one_small_request_to_each_provider`, `openai_compatible_sends_chat_completions_with_a_bearer_token`
- [[Provider Module]] — from `from_config`

## See also

- [[_index]]
- [[Provider Module]]
