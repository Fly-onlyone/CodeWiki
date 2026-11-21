---
tags: [core, cw-core, module]
---

# Settings Module (cw-core)

> Settings the backend keeps for its user: which embedder serves semantic search, and the keys and addresses it needs.

## What it does

They live in `settings.json` in the data directory, which only the backend writes. Every setting also has an environment variable, and the variable wins: a setup made with variables (Docker, CI, a test) behaves the same whatever the file says. `Resolved` holds the outcome for each setting and says where it came from. API keys are stored encrypted (`secret.rs`).

## Source

- `crates/cw-core/src/settings.rs` — module `cw_core::settings`

## How it works

1. `resolved` in [[Settings Module (cw-server)]] and `Resolved::load` in [[Resolved]] call `load`. `load` calls `read`. `read` calls `file`.
2. `run` in [[Serve Module]] calls `key`.
3. `change` in [[Settings Module (cw-server)]] calls `key_at`.
4. `change` in [[Settings Module (cw-server)]] calls `update`. `update` calls `file`, then `read`, then `Error::io`, then `Error::external` (both in [[Error]]).
5. `run` in [[Serve Module]] calls `drop_retired`. `drop_retired` calls `read`, then `update`.

1 more entry point: `retire_builtin`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const FILE: &str` — the file's name in the data directory.
- [[Key]] — one setting: the environment variable that names it everywhere, and where it sits in the file.
- `pub const EMBED_PROVIDER: Key` — public constant; used by `ALL`, `EMBEDDER_KEYS`, `change`, `checked` and 1 more.
- `pub const OLLAMA_HOST: Key` — public constant; used by `ADDRESS_KEYS`, `ALL`, `EMBEDDER_KEYS`.
- `pub const LMSTUDIO_URL: Key` — public constant; used by `ADDRESS_KEYS`, `ALL`, `EMBEDDER_KEYS`.
- `pub const LLAMACPP_URL: Key` — public constant; used by `ADDRESS_KEYS`, `ALL`, `EMBEDDER_KEYS`.
- `pub const LEMONADE_URL: Key` — public constant; used by `ADDRESS_KEYS`, `ALL`, `EMBEDDER_KEYS`.
- `pub const MISTRALRS_URL: Key` — mistral.rs has no key setting: its server takes none by default.
- `pub const ALL: [Key; 12]` — every setting, in the order a page shows them.
- `pub fn key(env: &str) -> Option<Key>` — the setting an environment variable names.
- `pub fn key_at(section: &str, field: &str) -> Option<Key>` — the setting at `section.field`.
- [[Settings]] — what `settings.json` holds.
- `pub fn load(data_dir: &Path) -> Settings` — the saved settings. A missing or unreadable file gives none, so a damaged file never stops the backend.
- `update` — read the file, apply `change` and write it back. The new file replaces the old one in one step, so a reader never sees half of it. When `change` fails, nothing is written.
- `pub fn drop_retired(data_dir: &Path) -> Result<bool>` — take the settings in `RETIRED_SECTIONS` out of the file. Returns whether it held any. A file without them is not written, and neither is one that is missing or cannot be read.
- `pub fn retire_builtin(data_dir: &Path) -> Result<Retired>` — move a user of the removed built-in embedder to a working state. A saved `builtin` choice becomes `local` and its model name is cleared, since the name belonged to the built-in engine. `embed.device` leaves the file, and the built-in models in `<data_dir>/models` are deleted. A file that holds none of these is not written.
- `pub enum Source` — where the value of a setting comes from.
- `pub enum SecretState` — what is known about a secret without showing it.
- [[Resolved]] — the settings in effect: for each one, the environment variable when it is set, otherwise the saved value, otherwise the default.
- `pub fn process_env(name: &str) -> Option<String>` — an environment variable of this process. What `Resolved::load` reads the environment with; a test passes its own function in its place.

And 16 more items.

## Depends on

- [[Error]] — uses `external`, `io`
- [[Key]] — uses `Key`, `path`
- [[Error Module]] — uses `Result`
- [[Settings]] — uses `Settings`

## Used by

- [[Settings Module (cw-server)]] — from `ADDRESS_KEYS`, `EMBEDDER_KEYS`, `change`, `checked`
- [[Resolved]] — from `Resolved`, `from_env`, `from_parts`, `load`
- [[Settings]] — from `clear`, `field`, `put`, `secret`
- [[Serve Module]] — from `run`

## See also

- [[_index]]
- [[cw-core]]
