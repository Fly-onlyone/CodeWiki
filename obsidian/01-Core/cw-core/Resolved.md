---
tags: [core, cw-core]
---

# Resolved

> The settings in effect: for each one, the environment variable when it is set, otherwise the saved value, otherwise the default.

## What it does

It holds decrypted secrets, so it has no `Debug` and should not outlive the work it was loaded for.

## Source

- `crates/cw-core/src/settings.rs` — definition of `Resolved`

## How it works

1. `run` in [[codewiki]] and `run` in [[Serve Module]] call `load`. `load` calls `from_parts`, then `load` in [[Settings Module (cw-core)]].
2. `change` and `resolved` in [[Settings Module (cw-server)]] call `from_parts`. `from_parts` calls `saved_secret`. `saved_secret` calls `unprotect` in [[Secret Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct Resolved`.

Members:

- `pub fn load(data_dir: &Path) -> Self` — the settings in effect for this process: its environment over the file in `data_dir`.
- `pub fn from_env() -> Self` — the environment alone, as before there was a settings file.
- `pub fn from_parts(saved: &Settings, env: impl Fn(&str) -> Option<String>) -> Self` — resolve `saved` under the environment `env` reads. A variable that is blank counts as not set.
- `fn saved_secret(saved: &Settings, key: Key) -> Entry` — private method, returns `Entry`; called by `Resolved::from_parts`.
- `pub fn get(&self, env: &str) -> Option<String>` — the value in effect for the setting an environment variable names. `None` for a name that is not a setting, and for a setting with neither a value nor a default.
- `pub fn source(&self, key: Key) -> Source` — where the value of `key` comes from.
- `pub fn secret_state(&self, key: Key) -> SecretState` — what is known about the secret `key`.

## Depends on

- [[Settings Module (cw-core)]] — uses `ALL`, `Entry`, `SecretState`, `Source`
- [[Settings]] — uses `Settings`, `secret`, `value`
- [[Secret Module]] — uses `unprotect`
- [[Key]] — uses `Key`

## Used by

- [[Settings Module (cw-server)]] — from `change`, `describe`, `resolved`
- [[cw-embed]] — from `from_settings`, `list_models_in`
- [[codewiki]] — from `run`
- [[Serve Module]] — from `run`
- [[Logs Module]] — from `get`
- [[Semantic Module]] — from `configure`
- [[Slot]] — from `configure`

## See also

- [[_index]]
- [[Settings Module (cw-core)]]
