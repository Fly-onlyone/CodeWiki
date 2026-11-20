---
tags: [core, cw-llm]
---

# AgentCli

> An installed agent CLI run in non-interactive mode.

## What it does

Uses the user's own subscription and login; no API key is involved.

## Source

- `crates/cw-llm/src/provider.rs` — definition of `AgentCli`

## How it works

1. `from_config` in [[Provider Module]] calls `claude`.
2. `from_config` in [[Provider Module]] calls `codex`.

## Items

Declared as `pub struct AgentCli`.

Members:

- `pub fn claude() -> Self` — Claude Code in print mode.
- `pub fn codex() -> Self` — Codex in non-interactive mode, reading the prompt from standard input.
- `fn name(&self) -> String` — public method, returns `String`.
- `fn complete(&self, system: &str, user: &str) -> Result<Completion>` — public method, returns `Result<Completion>`.

## Depends on

- [[Error Module]] — uses `Result`
- [[Provider Module]] — uses `Completion`
- [[Error]] — uses `external`
- [[LlmProvider]] — uses `LlmProvider`

## Used by

- [[Provider Module]] — from `from_config`

## See also

- [[_index]]
- [[Provider Module]]
