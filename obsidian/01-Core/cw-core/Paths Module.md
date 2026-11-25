---
tags: [core, cw-core, module]
---

# Paths Module

> Locations of per-user data.

## Source

- `crates/cw-core/src/paths.rs` — module `cw_core::paths`

## How it works

1. `agents` and `run` in [[codewiki]], and 24 more, call `data_dir`. `data_dir` calls `data_dir_from`.
2. `add` in [[Repos Module]] calls `repo_id`. `repo_id` calls `sha256_hex` in [[Hash Module]].
3. `open_index` in [[Service Module]] calls `index_path`. `index_path` calls `index_path_in`, then `data_dir`.
4. `open_existing_index_in` and `open_index_in` in [[Service Module]] call `index_path_in`. `index_path_in` calls `repo_id`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const DATA_DIR_ENV: &str` — environment variable that overrides the data directory (used in Docker).
- `pub const APP_FOLDER: &str` — the name of CodeWiki's folders under `%LOCALAPPDATA%` and `%APPDATA%`. A development build has folders of its own (`crate::DEV_BUILD`): a run from the sources would otherwise use, and on Quit stop, the backend of an installed CodeWiki, and read and write its index and settings.
- `const UNIX_FOLDER: &str` — `APP_FOLDER` as it is spelled under `$XDG_DATA_HOME` and `~/.local/share`.
- `pub fn data_dir() -> PathBuf` — directory for indexes and backend state. `CODEWIKI_DATA_DIR` when set, otherwise `%LOCALAPPDATA%\CodeWiki` on Windows and `$XDG_DATA_HOME/codewiki` or `~/.local/share/codewiki` elsewhere. A development build uses `CodeWiki-Dev` and `codewiki-dev` instead (`APP_FOLDER`).
- `fn data_dir_from(env: impl Fn(&str) -> Option<OsString>) -> PathBuf` — `data_dir`, with the environment read through `env`.
- `pub fn repo_id(root: &Path) -> String` — stable identifier for a repository: its folder name plus a short hash of the full path, so two checkouts with the same name do not collide.
- `pub fn index_path(root: &Path) -> PathBuf` — path of the index database for a repository.
- `pub fn index_path_in(data_dir: &Path, root: &Path) -> PathBuf` — path of the index database for a repository under a given data directory.

## Depends on

- [[cw-core]] — uses `DEV_BUILD`
- [[Hash Module]] — uses `sha256_hex`

## Used by

- [[codewiki-app]] — from `keep_agents_out`, `let_agents_in`, `open_settings`, `read_auto_explain`
- [[cw-client]] — from `check_remote`, `forget_remote`, `save_remote`, `saved_remote`
- [[codewiki]] — from `agents`, `run`, `sentry_test`, `session_start_hook`
- [[Service Module]] — from `open_existing_index_in`, `open_index`, `open_index_in`, `update`
- [[codewiki-mcp]] — from `listed_folders`, `screen`
- [[Settings View Module]] — from `new`, `read_agents_off`
- [[Backend]] — from `connect`, `find`
- [[cw-telemetry]] — from `consent_path`

## See also

- [[_index]]
- [[cw-core]]
