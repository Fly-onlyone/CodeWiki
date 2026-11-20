---
tags: [core, cw-server, module]
---

# Cliproxy Module (cw-server)

> What the CLIProxyAPI route does: describe the proxy the settings name.

## What it does

The user runs the proxy and manages its accounts in it, through its own web panel or its `-claude-login` and `-codex-login` flags. This backend only asks whether it answers and which models its accounts serve, with the client key. A backend that listens beyond its machine answers no such route, as with the settings.

## Source

- `crates/cw-server/src/cliproxy.rs` — module `cw_server::cliproxy`

## How it works

1. `read_cliproxy` in [[Api Module]] calls `describe`. `describe` calls `resolved` in [[Settings Module (cw-server)]], then `url`, then `panel`, then `reachable`, then `models` (all in [[Cliproxy Module]]), then `message`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `fn url(resolved: &Resolved) -> String` — private function, returns `String`; called by `describe`.
- `fn message(error: &Error) -> String` — an error's text for a person.
- `pub(crate) fn describe(state: &AppState) -> Value` — the proxy as the settings name it: whether it answers, the models its accounts serve, and where its own panel is. A part that cannot be read says why in its own error field.

## Depends on

- [[Cliproxy Module]] — uses `models`, `panel`, `reachable`
- [[Settings Module (cw-core)]] — uses `CLIPROXY_API_KEY`, `CLIPROXY_URL`
- [[Settings Module (cw-server)]] — uses `resolved`
- [[Error]] — uses `Error`
- [[Resolved]] — uses `Resolved`
- [[AppState]] — uses `AppState`

## Used by

- [[Api Module]] — from `read_cliproxy`

## See also

- [[_index]]
- [[cw-server]]
