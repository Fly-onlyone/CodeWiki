---
tags: [core, cw-core, module]
---

# Paths Module

> Locations of per-user data.

## Source

- `crates/cw-core/src/paths.rs` — module `cw_core::paths`

## How it works

Main items:

- `data_dir` — directory for indexes and backend state.
- `DATA_DIR_ENV` — environment variable that overrides the data directory (used in Docker).
- `repo_id` — stable identifier for a repository: its folder name plus a short hash of the full path, so two checkouts with the same name do not collide.
- `index_path_in` — path of the index database for a repository under a given data directory.
- `index_path` — path of the index database for a repository.

## Depends on

- [[Hash Module]] — uses `sha256_hex`

## Used by

- [[cw-client]] — from `check_remote`, `forget_remote`, `save_remote`, `saved_remote`
- [[Service Module]] — from `open_index`, `open_index_in`, `update`
- [[codewiki-app]] — from `run_settings_action`, `settings_frame`
- [[codewiki]] — from `run`
- [[Serve Module]] — from `run`
- [[Logs Module]] — from `dir_from`
- [[Repos Module]] — from `add`
- [[Backend]] — from `connect`

## See also

- [[_index]]
- [[cw-core]]
