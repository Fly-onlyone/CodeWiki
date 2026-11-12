---
tags: [core, cw-llm]
---

# Anthropic

> `Anthropic` struct defined in `cw_llm::provider` with 3 methods.

## Source

- `crates/cw-llm/src/provider.rs` — definition of `Anthropic`

## How it works

Declared as `pub struct Anthropic`.

Public API:

- `pub fn new(api_key: String, model: Option<String>) -> Self`
- `fn name(&self) -> String`
- `fn complete(&self, system: &str, user: &str) -> Result<Completion>`

## Depends on

- [[Provider Module]] — uses `ANTHROPIC_FALLBACK_BETA`, `ANTHROPIC_VERSION`, `Completion`, `DEFAULT_ANTHROPIC_MODEL`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `external`
- [[LlmProvider]] — uses `LlmProvider`

## Used by

- [[Provider Module]] — from `from_env`

## See also

- [[_index]]
- [[Provider Module]]
