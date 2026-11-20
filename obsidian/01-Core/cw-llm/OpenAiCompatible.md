---
tags: [core, cw-llm]
---

# OpenAiCompatible

> Any server speaking the OpenAI chat-completions protocol.

## Source

- `crates/cw-llm/src/provider.rs` — definition of `OpenAiCompatible`

## How it works

What the functions of this type call:

1. `probe` calls `send`. `send` calls `chat_request`, then `post_json` (both in [[Provider Module]]).
2. `complete` calls `send`, then `chat_completion` in [[Provider Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct OpenAiCompatible`.

Members:

- `fn send(&self, system: &str, user: &str, timeout: Duration) -> Result<Value>` — private method, returns `Result<Value>`; called by `OpenAiCompatible::complete`, `OpenAiCompatible::probe`.
- `fn name(&self) -> String` — public method, returns `String`.
- `fn probe(&self) -> Result<()>` — no output limit is sent: servers of this protocol do not agree on the name of that field. The question keeps the answer short.
- `fn complete(&self, system: &str, user: &str) -> Result<Completion>` — public method, returns `Result<Completion>`.

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
