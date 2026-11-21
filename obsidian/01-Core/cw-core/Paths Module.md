---
tags: [core, cw-core, module]
---

# Paths Module

> Locations of per-user data.

## Source

- `crates/cw-core/src/paths.rs` — module `cw_core::paths`

## How it works

1. `run` and `sentry_test` in [[codewiki]], and 11 more, call `data_dir`.
2. `add` in [[Repos Module]] calls `repo_id`. `repo_id` calls `sha256_hex` in [[Hash Module]].
3. `open_index` in [[Service Module]] calls `index_path`. `index_path` calls `index_path_in`, then `data_dir`.
4. `open_existing_index_in` and `open_index_in` in [[Service Module]] call `index_path_in`. `index_path_in` calls `repo_id`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const DATA_DIR_ENV: &str` — environment variable that overrides the data directory (used in Docker).
- `pub fn data_dir() -> PathBuf` — directory for indexes and backend state. `CODEWIKI_DATA_DIR` when set, otherwise `%LOCALAPPDATA%\CodeWiki` on Windows and `$XDG_DATA_HOME/codewiki` or `~/.local/share/codewiki` elsewhere.
- `pub fn repo_id(root: &Path) -> String` — stable identifier for a repository: its folder name plus a short hash of the full path, so two checkouts with the same name do not collide.
- `pub fn index_path(root: &Path) -> PathBuf` — path of the index database for a repository.
- `pub fn index_path_in(data_dir: &Path, root: &Path) -> PathBuf` — path of the index database for a repository under a given data directory.

## Depends on

- [[Hash Module]] — uses `sha256_hex`

## Used by

- [[cw-client]] — from `check_remote`, `forget_remote`, `save_remote`, `saved_remote`
- [[Service Module]] — from `open_existing_index_in`, `open_index`, `open_index_in`, `update`
- [[codewiki]] — from `run`, `sentry_test`
- [[codewiki-app]] — from `run_settings_action`, `settings_frame`
- [[Serve Module]] — from `run`
- [[Logs Module]] — from `dir_from`
- [[Repos Module]] — from `add`
- [[Backend]] — from `connect`

## See also

- [[_index]]
- [[cw-core]]
