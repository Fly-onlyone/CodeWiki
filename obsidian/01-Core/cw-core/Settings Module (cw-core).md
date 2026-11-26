---
tags: [core, cw-core, module]
---

# Settings Module (cw-core)

> Settings the backend keeps for its user: which embedder serves semantic search, and the keys and addresses it needs.

## What it does

Defines the user's settings for the backend and how they are stored. The settings choose which embedder serves semantic search and hold the addresses and API keys of the embedding servers (OpenAI, Ollama, LM Studio, llama.cpp, Lemonade, mistral.rs). They live in `settings.json` in the data directory. Only the backend writes that file. Each setting also has an environment variable that overrides the file, so a Docker or CI setup behaves the same whatever is saved.

## Source

- `crates/cw-core/src/settings.rs` — module `cw_core::settings`

## How it works

Each setting is a [[Key]] constant: its variable name, its place (`section.field`) in the file, whether it is a secret, and its default. `ALL` lists the 12 in page order. `key` finds one by variable name, `key_at` by section and field.

`load` reads the file into [[Settings]], and gives an empty one when the file is missing or damaged, so a bad file never stops the backend. `update` is the only writer: it reads, runs a change function, removes retired sections, stamps the version and writes through a temporary file and a rename. If the change fails, nothing is written. If the existing file cannot be parsed, it is first moved to `settings.broken.json`.

[[Resolved]] combines the environment, the file and the defaults.

Two cleanups run at server start. `drop_retired` removes the old `llm`, `anthropic` and `cliproxy` sections, which hold encrypted keys that no program uses. `retire_builtin` moves a user of the removed built-in embedder to `local`, clears the model name and the device entry, and deletes the downloaded built-in models.

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
- `pub fn retire_builtin(data_dir: &Path) -> Result<Retired>` — move a user of the removed built-in embedder to a working state. A saved `builtin` choice becomes `local` and its model name is cleared, since the name belonged to the built-in engine. `embed.device` leaves the file, and the built-in models in `<data_dir>/models` are deleted (`delete_models`). A file that holds none of these is not written.
- `pub enum Source` — where the value of a setting comes from.
- `pub enum SecretState` — what is known about a secret without showing it.
- [[Resolved]] — the settings in effect: for each one, the environment variable when it is set, otherwise the saved value, otherwise the default.
- `pub fn process_env(name: &str) -> Option<String>` — an environment variable of this process. What `Resolved::load` reads the environment with; a test passes its own function in its place.

And 17 more items.

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

## Gotchas

- Unknown parts of the file are kept as they are, so an older program does not erase what a newer one wrote.
- `delete_models` removes only folders with a `manifest.json`, plus `.partial`. Other files in `models` stay, since a data folder may be shared.
- Retired environment variables are listed in `RETIRED_VARIABLES`. A program that finds one set says so in its log.
- mistral.rs has a URL setting but no key setting. Its server needs none by default.
- The default ports of LM Studio and mistral.rs are the same, so the two collide if both run on one machine.

## See also

- [[_index]]
- [[cw-core]]
