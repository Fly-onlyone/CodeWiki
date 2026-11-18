---
tags: [core, cw-llm]
---

# Anthropic

> `Anthropic` struct defined in `cw_llm::provider` with 5 methods.

## Source

- `crates/cw-llm/src/provider.rs` — definition of `Anthropic`

## How it works

Declared as `pub struct Anthropic`.

Public API:

- `fn probe(&self) -> Result<()>` — any answer proves the key and the model, one cut off at the small limit included.
- `pub fn new(api_key: String, model: Option<String>) -> Self`
- `fn name(&self) -> String`
- `fn complete(&self, system: &str, user: &str) -> Result<Completion>`

## Depends on

- [[Provider Module]] — uses `ANTHROPIC_FALLBACK_BETA`, `ANTHROPIC_VERSION`, `Completion`, `DEFAULT_ANTHROPIC_MODEL`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `external`
- [[LlmProvider]] — uses `LlmProvider`

## Used by

- [[cw-llm]] — from `a_failed_probe_carries_the_providers_explanation`, `a_probe_is_one_small_request_to_each_provider`, `anthropic_reports_refusals_and_http_errors`, `anthropic_sends_the_messages_request_and_reads_text_blocks`
- [[Provider Module]] — from `from_config`

## See also

- [[_index]]
- [[Provider Module]]
