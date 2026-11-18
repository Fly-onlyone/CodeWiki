---
tags: [core, codewiki-app, module]
---

# Startup Module

> Which repository the app opens when it starts.

## Source

- `crates/codewiki-app/src/startup.rs` — module `codewiki_app::startup`

## How it works

Main items:

- `launch` — what this process was started with.
- `Launch` — what the command line asks for.
- `looks_like_repo` — whether `dir` looks like a repository: it holds `.git` (a folder, or a file in a git worktree or submodule) or a CodeWiki vault.
- `recent` — the folders of the repositories opened in the app, newest first.
- `repo` — the folder to open at startup, if any.
- `keep_recent` — drop from the recent list every folder that is not `registered`, and return what is left.
- `named_repo` — the folder a launch with the argument `arg` from the working folder `cwd` names, as a full path: the argument, or the working folder when it looks like a repository.
- `remember` — remember `path` as the repository to open next time, and put it first in the recent list.

## Depends on

- [[cw-core]] — uses `MANIFEST_DIR`, `VAULT_DIR`
- [[Prefs Module]] — uses `read`, `write`

## Used by

- [[codewiki-app]] — from `connect`, `listen`, `main`, `new`
- [[Instance Module]] — from `acquire`, `new`

## See also

- [[_index]]
- [[codewiki-app]]
