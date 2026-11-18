---
tags: [core, cw-core]
---

# Resolved

> The settings in effect: for each one, the environment variable when it is set, otherwise the saved value, otherwise the default.

## Source

- `crates/cw-core/src/settings.rs` — definition of `Resolved`

## How it works

Declared as `pub struct Resolved`.

Public API:

- `pub fn from_parts(saved: &Settings, env: impl Fn(&str) -> Option<String>) -> Self` — resolve `saved` under the environment `env` reads.
- `pub fn from_env() -> Self` — the environment alone, as before there was a settings file.
- `pub fn get(&self, env: &str) -> Option<String>` — the value in effect for the setting an environment variable names.
- `pub fn load(data_dir: &Path) -> Self` — the settings in effect for this process: its environment over the file in `data_dir`.
- `pub fn secret_state(&self, key: Key) -> SecretState` — what is known about the secret `key`.
- `pub fn source(&self, key: Key) -> Source` — where the value of `key` comes from.

## Depends on

- [[Settings Module (cw-core)]] — uses `ALL`, `Entry`, `SecretState`, `Source`
- [[Settings]] — uses `Settings`, `secret`, `value`
- [[Secret Module]] — uses `unprotect`
- [[Key]] — uses `Key`

## Used by

- [[Settings Module (cw-server)]] — from `change`, `describe`, `resolved`
- [[Serve Module]] — from `run`, `writer_name`
- [[codewiki]] — from `run`
- [[cw-embed]] — from `from_settings`
- [[cw-llm]] — from `writer_from`
- [[Logs Module]] — from `get`
- [[Provider Module]] — from `from_settings`
- [[Semantic Module]] — from `configure`

## See also

- [[_index]]
- [[Settings Module (cw-core)]]
