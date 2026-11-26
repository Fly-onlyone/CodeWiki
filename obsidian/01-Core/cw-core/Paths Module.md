---
tags: [core, cw-core, module]
---

# Paths Module

> Locations of per-user data.

## What it does

Says where CodeWiki keeps per-user files and which file holds a repository's index. Every program (app, backend, MCP bridge, command line) calls `data_dir`, so they all find the same settings, logs, `backend.json` and indexes.

## Source

- `crates/cw-core/src/paths.rs` — module `cw_core::paths`

## How it works

`data_dir` picks the first that exists: `CODEWIKI_DATA_DIR` (used in Docker), then `%LOCALAPPDATA%` plus the app folder, then `$XDG_DATA_HOME` or `~/.local/share` plus a lower-case folder, and as a last resort `.codewiki-data` in the working folder. Empty variables count as unset.

`repo_id` names a repository from its folder name plus the first 8 hex characters of a SHA-256 ([[Hash Module]]) of its full path. The path is made canonical, lower-cased and given forward slashes first, so `C:\Code\My Repo` and `c:/code/my repo/` give the same id. Characters other than letters, digits, dash and underscore in the name become dashes. `index_path` is `<data dir>/index/<repo id>.db`.

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

- [[codewiki-app]] — from `keep_agents_out`, `let_agents_in`, `read_auto_explain`, `reload_auto_explain`
- [[cw-client]] — from `check_remote`, `forget_remote`, `save_remote`, `saved_remote`
- [[codewiki]] — from `agents`, `run`, `sentry_test`, `session_start_hook`
- [[Service Module]] — from `open_existing_index_in`, `open_index`, `open_index_in`, `update`
- [[codewiki-mcp]] — from `listed_folders`, `screen`
- [[Settings View Module]] — from `new`, `read_agents_off`
- [[Backend]] — from `connect`, `find`
- [[cw-telemetry]] — from `consent_path`

## Gotchas

- A development build uses `CodeWiki-Dev` or `codewiki-dev` folders. Without this, a run from the sources would use, and on Quit stop, an installed CodeWiki's backend and share its index and settings.
- `repo_id` is path-based. Moving a repository gives a new id and a new index.
- Case is folded, so two paths that differ only in case share an id, even on a case-sensitive disk.
- If `canonicalize` fails (the folder is gone), the path as given is used.

## See also

- [[_index]]
- [[cw-core]]
