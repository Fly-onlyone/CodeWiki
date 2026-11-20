---
tags: [core, cw-llm, module]
---

# Cliproxy Module

> CLIProxyAPI: a server the user runs that holds many Claude and Codex subscription logins and serves them behind one OpenAI-compatible API, taking the next account for each request and resting one that reached its limit (github.com/router-for-me/CLIProxyAPI).

## What it does

CodeWiki never starts, stops or installs it, and leaves its accounts to it: they are added and removed in the proxy's own web panel or with its `-claude-login` and `-codex-login` flags. `CliProxy` writes prose through it with a client key; `models` lists what its accounts serve. The shapes here were read from a live CLIProxyAPI 8.0.79 and from its source.

## Source

- `crates/cw-llm/src/cliproxy.rs` — module `cw_llm::cliproxy`

## How it works

1. `CliProxy::send` in [[CliProxy]] calls `unreachable`. `unreachable` calls `base`.
2. `CliProxy::send` in [[CliProxy]] calls `base`.
3. `describe` in [[Cliproxy Module (cw-server)]] calls `panel`. `panel` calls `base`.
4. `CliProxy::send` in [[CliProxy]] calls `exhausted`.
5. `describe` in [[Cliproxy Module (cw-server)]] calls `reachable`. `reachable` calls `exchange` in [[Provider Module]], then `unreachable`, then `base`.

1 more entry point: `models`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const LIST_TIMEOUT: Duration` — how long the model list waits: it is small.
- `const REACH_TIMEOUT: Duration` — how long `reachable` waits: a local server answers at once.
- `const NO_ACCOUNT: [&str; 2]` — error codes with which the proxy says no account can take a request (`isAuthSelectionUnavailable` in its `sdk/api/handlers/handlers_errors.go`).
- `fn unreachable(base_url: &str) -> String` — what an error says when nothing answers at `base_url`.
- `fn base(base_url: &str) -> &str` — private function, returns `&str`; called by `CliProxy::send`, `models`, `panel`, `reachable` and 1 more.
- `pub fn panel(base_url: &str) -> String` — the proxy's own control panel, where its accounts are managed. It is there unless the proxy's `management.disable-control-panel` is set, and asks for the proxy's `management.secret-key`.
- `fn exhausted(reply: &Reply) -> Option<Error>` — whether the proxy's answer means no account can answer now:
  - a 429 after it has tried its accounts;
  - a 503 naming an empty pool;
  - a 400 `model_not_found`. The proxy knows a model only through an account that serves it, so this is what it says once that account is removed or disabled (seen on CLIProxyAPI 8.0.79).
- [[CliProxy]] — writes prose through a CLIProxyAPI server with chat completions.
- `pub fn reachable(base_url: &str) -> bool` — whether anything answers at `base_url`: any HTTP answer will do, a refused key included.
- `pub fn models(base_url: &str, api_key: &str) -> Result<Vec<String>>` — the models the proxy can serve with the accounts it holds, sorted.

And 1 more private item with no comment.

## Depends on

- [[Provider Module]] — uses `Call`, `Reply`, `exchange`, `http_error`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `Error`

## Used by

- [[cw-llm]] — from `a_proxy_that_does_not_answer_is_named`, `any_answer_means_the_proxy_is_there`, `models_are_listed_with_the_client_key`
- [[Cliproxy Module (cw-server)]] — from `describe`
- [[CliProxy]] — from `send`

## See also

- [[_index]]
- [[cw-llm]]
