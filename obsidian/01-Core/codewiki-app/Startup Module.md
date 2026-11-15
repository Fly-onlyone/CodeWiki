---
tags: [core, codewiki-app, module]
---

# Startup Module

> Which repository the app opens when it starts.

## Source

- `crates/codewiki-app/src/startup.rs` — module `codewiki_app::startup`

## How it works

Main items:

- `looks_like_repo` — whether `dir` looks like a repository: it holds `.git` (a folder, or a file in a git worktree or submodule) or a CodeWiki vault.
- `recent` — the folders of the repositories opened in the app, newest first.
- `keep_recent` — drop from the recent list every folder that is not `registered`, and return what is left.
- `remember` — remember `path` as the repository to open next time, and put it first in the recent list.
- `repo` — the folder to open at startup, if any.
- `RECENT_CAP` — how many folders the recent list keeps.
- `RECENT_REPOS` — the setting that holds the recent list, one folder per line.
- `parse` — the recent list in the setting's text: one folder per line.

## Depends on

- [[cw-core]] — uses `MANIFEST_DIR`, `VAULT_DIR`
- [[Prefs Module]] — uses `read`, `write`

## Used by

- [[codewiki-app]] — from `connect`, `new`, `select_repo`, `take_repos`

## See also

- [[_index]]
- [[codewiki-app]]
