---
tags: [core, cw-llm, module]
---

# Provider Module

> LLM providers: hosted APIs and locally installed agent CLIs.

## Source

- `crates/cw-llm/src/provider.rs` — module `cw_llm::provider`

## How it works

1. `writer_from` in [[cw-llm]] and `writer_name` in [[Serve Module]], and 3 more, call `from_settings`. `from_settings` calls `from_config`. `from_config` calls `setting`, then `missing`, then `Anthropic::new` in [[Anthropic]], then `AgentCli::claude`, then `AgentCli::codex` (both in [[AgentCli]]), then `Error::invalid` in [[Error]].
2. `Anthropic::send` in [[Anthropic]] and `Ollama::send` in [[Ollama]], and 1 more, call `post_json`. `post_json` calls `exchange`, then `http_error`.
3. `models` and `reachable` in [[Cliproxy Module]], and 1 more, call `exchange`. `exchange` calls `agent`, then `Error::external` in [[Error]].
4. `models` in [[Cliproxy Module]] and `CliProxy::send` in [[CliProxy]] call `http_error`. `http_error` calls `Error::external` in [[Error]].
5. `CliProxy::send` in [[CliProxy]] and `OpenAiCompatible::send` in [[OpenAiCompatible]] call `chat_request`.

1 more entry point: `chat_completion`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const DEFAULT_ANTHROPIC_MODEL: &str` — default Anthropic model for prose generation.
- `pub struct Completion` — public struct; used by `AgentCli::complete`, `Anthropic::complete`, `CliProxy::complete`, `LlmProvider::complete` and 5 more.
- [[LlmProvider]] — trait
- `pub(crate) enum Method` — internal enum; used by `Call`.
- `pub(crate) struct Call<'a>` — one HTTP request to a provider.
- `pub(crate) struct Reply` — whatever the server answered, an error status included.
- `pub(crate) fn exchange(call: &Call) -> Result<Reply>` — send one request. Only a server that cannot be reached is an error here; an HTTP error status is a `Reply` like any other.
- `pub(crate) fn http_error(provider: &str, reply: &Reply) -> Error` — the error for an HTTP error status, with the provider's own explanation.
- `pub(crate) fn chat_request(model: &str, system: &str, user: &str) -> Value` — the body of a chat-completions request.
- `pub(crate) fn chat_completion(value: &Value) -> Completion` — the answer in a chat-completions response.
- [[Anthropic]] — struct
- [[OpenAiCompatible]] — any server speaking the OpenAI chat-completions protocol.
- [[Ollama]] — struct
- [[AgentCli]] — an installed agent CLI run in non-interactive mode.
- `pub const PROVIDER_ENV: &str` — environment variable selecting the provider: one of `PROVIDERS`.
- `pub const MODEL_ENV: &str` — model name for the selected provider.
- `pub const PROVIDERS: [&str; 7]` — the providers a user can choose, in the order a page lists them.
- `from_config` — build the provider that `lookup` describes. `lookup` reads a setting by the name of its environment variable. `Ok(None)` means keyless mode: no provider is configured, which is a supported way to run.
- `pub fn from_env() -> Result<Option<Box<dyn LlmProvider>>>` — the provider chosen by the environment alone.
- `pub fn from_settings(resolved: &Resolved) -> Result<Option<Box<dyn LlmProvider>>>` — the provider chosen by the settings in effect: the environment over the settings file.

And 14 more items.

## Depends on

- [[Settings Module (cw-core)]] — uses `ANTHROPIC_API_KEY`, `CLIPROXY_API_KEY`, `CLIPROXY_URL`, `LLM_MODEL`
- [[Error]] — uses `Error`, `external`, `invalid`
- [[AgentCli]] — uses `claude`, `codex`
- [[Error Module]] — uses `Result`
- [[Key]] — uses `Key`
- [[Resolved]] — uses `Resolved`
- [[CliProxy]] — uses `CliProxy`
- [[Anthropic]] — uses `new`

## Used by

- [[Anthropic]] — from `complete`, `new`, `probe`, `send`
- [[Cliproxy Module]] — from `exhausted`, `models`, `reachable`
- [[Settings Module (cw-server)]] — from `checked`, `describe`, `test_llm`
- [[CliProxy]] — from `complete`, `probe`, `send`
- [[Ollama]] — from `complete`, `probe`, `send`
- [[OpenAiCompatible]] — from `complete`, `probe`, `send`
- [[Api Module]] — from `ask`, `span_status`
- [[LlmProvider]] — from `complete`, `probe`

## See also

- [[_index]]
- [[cw-llm]]
