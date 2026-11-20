---
tags: [core, cw-llm]
---

# CliProxy

> Writes prose through a CLIProxyAPI server with chat completions.

## Source

- `crates/cw-llm/src/cliproxy.rs` — definition of `CliProxy`

## How it works

What the functions of this type call:

1. `probe` calls `send`. `send` calls `chat_request`, then `exchange` (both in [[Provider Module]]), then `unreachable`, then `base`, then `exhausted` (all in [[Cliproxy Module]]), then `http_error` in [[Provider Module]].
2. `complete` calls `send`, then `chat_completion` in [[Provider Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct CliProxy`.

Members:

- `fn send(&self, system: &str, user: &str, timeout: Duration) -> Result<Value>` — private method, returns `Result<Value>`; called by `CliProxy::complete`, `CliProxy::probe`.
- `fn name(&self) -> String` — public method, returns `String`.
- `fn probe(&self) -> Result<()>` — public method, returns `Result<()>`.
- `fn complete(&self, system: &str, user: &str) -> Result<Completion>` — public method, returns `Result<Completion>`.

## Depends on

- [[Provider Module]] — uses `Call`, `Completion`, `PROBE_SYSTEM`, `PROBE_TIMEOUT`
- [[Cliproxy Module]] — uses `NAME`, `base`, `exhausted`, `unreachable`
- [[Error Module]] — uses `Result`
- [[LlmProvider]] — uses `LlmProvider`

## Used by

- [[cw-llm]] — from `proxy`
- [[Provider Module]] — from `from_config`

## See also

- [[_index]]
- [[Cliproxy Module]]
