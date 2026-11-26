---
tags: [core, cw-core]
---

# Resolved

> The settings in effect: for each one, the environment variable when it is set, otherwise the saved value, otherwise the default.

## What it does

The answer to "what value does this setting have right now?" For each of the 12 settings it holds the value in effect and where it came from: the environment, the saved file, or the default. Code that needs a setting, such as the embedder or a server address, reads it here, not from the file or the environment directly. The [[Settings Module (cw-core)]] note lists the settings.

## Source

- `crates/cw-core/src/settings.rs` — definition of `Resolved`

## How it works

`from_parts` goes through every [[Key]]. A set, non-blank environment variable wins (`Source::Env`). Otherwise a saved value from [[Settings]] is used (`Source::Saved`). Otherwise the key's default is used, which may be none (`Source::Default`).

Secrets are handled apart. If the variable is not set, `saved_secret` decrypts the stored value with [[Secret Module]]. The `SecretState` records unset, from the environment, saved with a hint, or saved but unreadable.

`load` reads the file in the data directory and the real environment. `from_env` ignores the file. `get` looks a value up by environment variable name. `source` and `secret_state` tell a settings page what to show. `from_parts` takes the environment as a function, so tests pass their own.

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
- [[cw-server]] — from `no_model_server`
- [[cw-wiki]] — from `no_model_server`
- [[Serve Module]] — from `run`
- [[Logs Module]] — from `get`
- [[Semantic Module]] — from `configure`

## Gotchas

- It holds decrypted secrets. It has no `Debug` on purpose, so it cannot be printed by accident. Keep it only as long as needed.
- A blank variable counts as not set. A variable of spaces does not hide the saved value.
- A secret that cannot be decrypted gives no value, but the state is `Unreadable`, not `Unset`. A page can then ask the user to enter it again.
- The embedder model has no default of its own, since it depends on the provider.

## See also

- [[_index]]
- [[Settings Module (cw-core)]]
