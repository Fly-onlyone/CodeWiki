---
tags: [core, cw-core, module]
---

# Settings Module (cw-core)

> Settings the backend keeps for its user: which LLM writes prose, which embedder serves semantic search, and the keys and addresses they need.

## Source

- `crates/cw-core/src/settings.rs` — module `cw_core::settings`

## How it works

Main items:

- [[Key]] — one setting: the environment variable that names it everywhere, and where it sits in the file.
- [[Resolved]] — the settings in effect: for each one, the environment variable when it is set, otherwise the saved value, otherwise the default.
- [[Settings]] — what `settings.json` holds.
- `SecretState` — what is known about a secret without showing it.
- `key` — the setting an environment variable names.
- `Source` — where the value of a setting comes from.
- `ALL` — every setting, in the order a page shows them.
- `FILE` — the file's name in the data directory.

## Depends on

- [[Error]] — uses `external`, `io`
- [[Error Module]] — uses `Result`
- [[Key]] — uses `Key`
- [[Settings]] — uses `Settings`

## Used by

- [[Resolved]] — from `Resolved`, `from_env`, `from_parts`, `load`
- [[Settings]] — from `clear`, `field`, `put`, `secret`
- [[Settings Module (cw-server)]] — from `EMBEDDER_KEYS`, `change`, `checked`, `describe`
- [[Provider Module]] — from `MODEL_ENV`, `PROVIDER_ENV`, `from_config`
- [[Models Module]] — from `chosen`, `describe`, `switch`
- [[Model Module (codewiki)]] — from `remove`, `store`
- [[Serve Module]] — from `run`

## See also

- [[_index]]
- [[cw-core]]
