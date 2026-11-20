---
tags: [core, cw-core, module]
---

# Settings Module (cw-core)

> Settings the backend keeps for its user: which LLM writes prose, which embedder serves semantic search, and the keys and addresses they need.

## What it does

They live in `settings.json` in the data directory, which only the backend writes. Every setting also has an environment variable, and the variable wins: a setup made with variables (Docker, CI, a test) behaves the same whatever the file says. `Resolved` holds the outcome for each setting and says where it came from. API keys are stored encrypted (`secret.rs`).

## Source

- `crates/cw-core/src/settings.rs` — module `cw_core::settings`

## How it works

1. `resolved` in [[Settings Module (cw-server)]] and `Resolved::load` in [[Resolved]] call `load`. `load` calls `read`. `read` calls `file`.
2. `change` in [[Settings Module (cw-server)]] calls `key_at`.
3. `change` in [[Settings Module (cw-server)]] calls `update`. `update` calls `file`, then `read`, then `Error::io`, then `Error::external` (both in [[Error]]).
4. `store` in [[Model Module (codewiki)]] calls `process_env`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const FILE: &str` — the file's name in the data directory.
- [[Key]] — one setting: the environment variable that names it everywhere, and where it sits in the file.
- `pub const LLM_PROVIDER: Key` — public constant; used by `ALL`, `PROVIDER_ENV`, `checked`, `from_config`.
- `pub const LLM_MODEL: Key` — public constant; used by `ALL`, `MODEL_ENV`, `from_config`.
- `pub const EMBED_PROVIDER: Key` — public constant; used by `ALL`, `EMBEDDER_KEYS`, `change`, `checked` and 4 more.
- `pub const EMBED_MODEL: Key` — public constant; used by `ALL`, `EMBEDDER_KEYS`, `change`, `chosen` and 2 more.
- `pub const EMBED_DEVICE: Key` — what runs a built-in embedding model: `auto` (the graphics card when it can, else the processor), `cpu` or `cuda`.
- `pub const OPENAI_BASE_URL: Key` — public constant; used by `ALL`, `EMBEDDER_KEYS`, `checked`, `from_config`.
- `pub const OLLAMA_HOST: Key` — public constant; used by `ALL`, `EMBEDDER_KEYS`, `checked`, `from_config`.
- `pub const CLIPROXY_URL: Key` — where a CLIProxyAPI server listens. The user runs it; CodeWiki only connects. Its default port is 8317.
- `pub const ALL: [Key; 11]` — every setting, in the order a page shows them.
- `pub fn key(env: &str) -> Option<Key>` — the setting an environment variable names.
- `pub fn key_at(section: &str, field: &str) -> Option<Key>` — the setting at `section.field`.
- [[Settings]] — what `settings.json` holds.
- `pub fn load(data_dir: &Path) -> Settings` — the saved settings. A missing or unreadable file gives none, so a damaged file never stops the backend.
- `update` — read the file, apply `change` and write it back. The new file replaces the old one in one step, so a reader never sees half of it. When `change` fails, nothing is written.
- `pub enum Source` — where the value of a setting comes from.
- `pub enum SecretState` — what is known about a secret without showing it.
- [[Resolved]] — the settings in effect: for each one, the environment variable when it is set, otherwise the saved value, otherwise the default.
- `pub fn process_env(name: &str) -> Option<String>` — an environment variable of this process. What `Resolved::load` reads the environment with; a test passes its own function in its place.

And 9 more items.

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
- [[Cliproxy Module (cw-server)]] — from `describe`, `url`
- [[Serve Module]] — from `run`

## See also

- [[_index]]
- [[cw-core]]
