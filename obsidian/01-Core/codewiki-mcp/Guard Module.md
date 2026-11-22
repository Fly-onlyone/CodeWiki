---
tags: [core, codewiki-mcp, module]
---

# Guard Module

> Which tool calls coding agents may make: folders closed to them in CodeWiki's settings get no tools, and cross-project tools do not show listed projects.

## What it does

Plain functions, no backend, so they test without one.

## Source

- `crates/codewiki-mcp/src/guard.rs` — module `codewiki_mcp::guard`

## How it works

1. `Bridge::screen` in [[codewiki-mcp]] calls `session_folders`. `session_folders` calls `normalize` in [[Agents Off Module]].
2. `Bridge::call` and `Bridge::screen` in [[codewiki-mcp]] call `repo_arg`. `repo_arg` calls `normalize` in [[Agents Off Module]].
3. `Bridge::screen` in [[codewiki-mcp]] calls `lookup_name`.
4. `Bridge::screen` in [[codewiki-mcp]] calls `resolve_id`. `resolve_id` calls `find_repo`.
5. `Bridge::call` in [[codewiki-mcp]] calls `path_of`.

4 more entry points: `direct_path`, `target`, `verdict`, `hide_repos`.

## Items

- `pub enum Verdict` — whether a tool call may go ahead.
- `pub fn session_folders(cwd: &Path, project_dir: Option<&Path>) -> Vec<PathBuf>` — the folders the session works in: the working directory, plus `CLAUDE_PROJECT_DIR` when set, made absolute against the working directory so that a relative value cannot slip past the list.
- [[RepoArg]] — A tool's `repo` argument: a folder path or a repository id or name.
- `pub fn repo_arg(value: &str, cwd: &Path) -> RepoArg` — A path when it is absolute, starts with a drive (`D:secret`, which Windows reads against that drive's current folder), starts with `.`, or contains a separator; otherwise an id or name. A path is made absolute against `cwd`, so a value with a drive is never taken for an id or name.
- `pub fn lookup_name(arg: &RepoArg, exists: impl Fn(&Path) -> bool) -> Option<String>` — the id or name to look up in `/v1/repos` for a `repo` argument, if any: an id or name as given, or the last component of a path that is not an existing folder. The backend falls back to that name when it cannot add the path (`cw-client`'s `repo_id`), so a missing path can still reach a listed repository. An existing folder needs no lookup.
- `fn find_repo<'a>(listing: &'a Value, id_or_name: &str) -> Option<&'a Value>` — the `/v1/repos` entry whose id equals `id_or_name` or whose name equals it case-insensitively (the rule of `cw-client`'s `repo_id`).
- `pub fn resolve_id(listing: &Value, id_or_name: &str) -> Option<PathBuf>` — the `path` of the entry `id_or_name` names.
- `pub fn path_of(listing: &Value, id: &str) -> Option<PathBuf>` — the `path` of the entry whose id is `id`; a name does not count.
- `pub fn direct_path(arg: &RepoArg, cwd: &Path) -> PathBuf` — the folder a `repo` argument means when no registered repository answers to it: a path as it is, a bare word as a path under `cwd`. The backend would read a bare word against its own folder, so it is never sent one.
- `pub enum Target` — what the backend is given for a `repo` argument.
- `pub fn target(arg: &RepoArg, listing: Option<&Value>, cwd: &Path) -> Target` — an id or name that a registered repository answers to is that repository's id; any other word is a folder under `cwd`; a path stays a path.
- `verdict` — refused when the list cannot be read, or when an entry covers a session folder or lies inside one; the refusal names that entry.
- `pub fn hide_repos(listing: &mut Value, entries: &[PathBuf]) -> HashSet<String>` — remove the entries of a `/v1/repos` array whose folder is covered or holds an entry; returns their names.
- `pub fn hide_in_workspace(workspace: &mut Value, hidden: &HashSet<String>)` — drop hidden repositories from a `/v1/workspace` view and from its shared dependencies (a dependency left with fewer than two repositories goes).
- `pub fn hide_in_contracts(contracts: &mut Value, hidden: &HashSet<String>)` — drop hidden repositories from a `/v1/workspace/contracts` view: rows of every list keyed by a hidden `repo`, and hidden providers of a matched call. A visible call left with no visible provider moves to `unprovided`, so the lists keep adding up to each repository's counts. Known limitation: a visible route called only by a hidden repository leaves `matched` and does not appear in `unconsumed`, because a provider reference has no file or line to rebuild the row from. Fixing it needs the server to compute over visible repositories only.

And 2 more private items with no comment.

## Depends on

- [[Agents Off Module]] — uses `blocking`, `normalize`, `refusal`, `unreadable`
- [[Error Module]] — uses `Result`
- [[RepoArg]] — uses `RepoArg`

## Used by

- [[codewiki-mcp]] — from `call`, `get_contracts`, `get_workspace`, `screen`

## See also

- [[_index]]
- [[codewiki-mcp]]
