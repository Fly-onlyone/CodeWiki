---
tags: [core, cw-server, module]
---

# Settings Module (cw-server)

> What the settings routes do: describe the settings in effect, and change the saved ones.

## What it does

A description never holds a secret: a key is reported as unset, saved, given by the environment, or unreadable, with the last characters of a saved one. A setting whose environment variable is set cannot be changed here; the variable wins, so a change would have no effect.

## Source

- `crates/cw-server/src/settings.rs` — module `cw_server::settings`

## How it works

1. `serve` in [[cw-server]] and `chosen` in [[Models Module]], and 2 more, call `resolved`. `resolved` calls `Resolved::from_parts` in [[Resolved]], then `load` in [[Settings Module (cw-core)]].
2. `write_settings` in [[Api Module]] and `remove` in [[Models Module]], and 1 more, call `change`. `change` calls `Error::invalid` in [[Error]], then `resolved`, then `update`, then `key_at` (both in [[Settings Module (cw-core)]]), then `apply`, then `Resolved::from_parts` in [[Resolved]], and 3 more. `apply` calls `Error::invalid` in [[Error]], then `checked`, then `available` in [[Secret Module]].
3. `read_settings` in [[Api Module]] calls `describe`. `describe` calls `resolved`, then `available` in [[Secret Module]], then `cuda_usable` in [[Worker Module]], then `selected`, then `from_settings_in` in [[cw-embed]]. `selected` calls `message`.
4. `test_embedder` in [[Api Module]] calls `test_embedder`. `test_embedder` calls `from_settings_in` in [[cw-embed]], then `resolved`, then `test_result`, then `Error::invalid` in [[Error]]. `test_result` calls `message`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const EMBEDDER_KEYS: [Key; 6]` — settings that decide which embedder semantic search uses.
- `pub(crate) fn resolved(state: &AppState) -> Resolved` — the settings in effect for this server.
- `fn message(error: &Error) -> String` — an error's text for a person: without the prefix that tells kinds of error apart.
- `selected` — the name of what the settings select, or why they select nothing usable.
- `pub(crate) fn describe(state: &AppState) -> Value` — every setting with its value in effect and where that comes from.
- `fn test_result(name: Option<String>, outcome: Result<Value>, started: Instant) -> Value` — what a test of a provider found: `ok`, the provider's name, and either how long it took (with `extra`, what the answer showed) or its error.
- `pub(crate) fn test_embedder(state: &AppState) -> Value` — have the chosen embedder embed one word. On success `dimensions` is the length of its vectors.
- `fn checked(key: Key, value: &str) -> Result<String>` — A value that passed the checks of its setting, as it is saved.
- `fn apply(saved: &mut Settings, key: Key, value: &Value) -> Result<()>` — apply one field of a change. `null` clears a setting. A plain setting takes text; a secret takes `{"set": "<secret>"}` or `{"remove": true}`.
- `pub(crate) fn change(state: &AppState, body: &Value) -> Result<Value>` — change the saved settings named in `body`, an object of sections holding fields, and return the new description. Fields not named stay as they are. One field that cannot be applied leaves everything unchanged.

## Depends on

- [[Settings Module (cw-core)]] — uses `ALL`, `EMBED_DEVICE`, `EMBED_MODEL`, `EMBED_PROVIDER`
- [[Resolved]] — uses `Resolved`, `from_parts`, `secret_state`, `source`
- [[cw-embed]] — uses `DEVICES`, `PROVIDERS`, `from_settings_in`
- [[Error]] — uses `Error`, `invalid`
- [[cw-models]] — uses `ENGINE`
- [[Error Module]] — uses `Result`
- [[Secret Module]] — uses `available`
- [[Worker Module]] — uses `cuda_usable`

## Used by

- [[Models Module]] — from `chosen`, `describe`, `remove`, `switch`
- [[Api Module]] — from `read_settings`, `test_embedder`, `write_settings`
- [[cw-server]] — from `serve`

## See also

- [[_index]]
- [[cw-server]]
