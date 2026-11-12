---
tags: [core, cw-llm]
---

# AgentCli

> An installed agent CLI run in non-interactive mode.

## Source

- `crates/cw-llm/src/provider.rs` — definition of `AgentCli`

## How it works

Declared as `pub struct AgentCli`.

Uses the user's own subscription and login; no API key is involved.

Public API:

- `pub fn claude() -> Self` — Claude Code in print mode.
- `pub fn codex() -> Self` — Codex in non-interactive mode, reading the prompt from standard input.
- `fn name(&self) -> String`
- `fn complete(&self, system: &str, user: &str) -> Result<Completion>`

## Depends on

- [[Error Module]] — uses `Result`
- [[Provider Module]] — uses `Completion`
- [[Error]] — uses `external`
- [[LlmProvider]] — uses `LlmProvider`

## Used by

- [[Provider Module]] — from `from_env`

## See also

- [[_index]]
- [[Provider Module]]
