---
tags: [core, codewiki-app, module]
---

# Startup Module

> Which repository the app opens when it starts.

## What it does

A folder given as the first argument is opened as given. Without one, the working folder counts only when it looks like a repository: started from the Start menu, the app's working folder is wherever the shortcut says (it once said the user's profile), and opening that folder would index whatever is in it. Otherwise the repository opened last, while its folder is still there. With none of these, the app opens the newest repository of the recent list that is still registered, or none: Home then lists the registered ones and offers to open a folder.

The app starts on Home, unless "Open the last repository" is on in the settings: it then starts on the wiki map of the repository it opens, and on Home still when that repository has no wiki yet.

The recent list holds the folders of the repositories opened in the app, newest first. Home orders the registered repositories by it.

The command line is `codewiki-app [--tray] [<folder> [<view>]]`. Flags may stand anywhere; the folder and the view are the first and second argument that is not a flag.

## Source

- `crates/codewiki-app/src/startup.rs` — module `codewiki_app::startup`

## How it works

1. `AppView::listen` and `AppView::new` in [[codewiki-app]], and 3 more, call `launch`. `launch` calls `parse_args`.
2. `Request::new` in [[Instance Module]] calls `named_repo`. `named_repo` calls `looks_like_repo`.
3. `AppView::open_startup_view` and `AppView::settings_frame` in [[codewiki-app]] call `open_last`. `open_last` calls `read` in [[Prefs Module]].
4. `AppView::run_action` in [[codewiki-app]] calls `set_open_last`. `set_open_last` calls `write` in [[Prefs Module]].
5. `AppView::connect` in [[codewiki-app]] calls `repo`. `repo` calls `choose`, then `launch`, then `read` in [[Prefs Module]]. `choose` calls `looks_like_repo`.

3 more entry points: `remember`, `recent`, `keep_recent`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub struct Launch` — what the command line asks for.
- `fn parse_args(args: impl IntoIterator<Item = String>) -> Launch` — read the arguments after the program's name. An argument that starts with `--` is a flag, and one this version does not know is left out: a flag is never taken for a folder, which the backend would index.
- `pub fn launch() -> &'static Launch` — what this process was started with.
- `pub fn named_repo(arg: Option<&str>, cwd: Option<&Path>) -> Option<String>` — the folder a launch with the argument `arg` from the working folder `cwd` names, as a full path: the argument, or the working folder when it looks like a repository. This is what a second launch hands to the running app, which has another working folder and so cannot read a relative path.
- `const LAST_REPO: &str` — the setting that remembers the repository opened last.
- `const RECENT_REPOS: &str` — the setting that holds the recent list, one folder per line.
- `const RECENT_CAP: usize` — how many folders the recent list keeps.
- `const OPEN_LAST: &str` — the setting that says whether the app starts on the wiki map of the repository it opens, rather than on Home.
- `pub fn open_last() -> bool` — whether the app starts on the wiki map of the repository it opens. Off until the user turns it on.
- `pub fn set_open_last(on: bool)` — public function; called by `AppView::run_action`.
- `pub fn looks_like_repo(dir: &Path) -> bool` — whether `dir` looks like a repository: it holds `.git` (a folder, or a file in a git worktree or submodule) or a CodeWiki vault.
- `fn choose(arg: Option<String>, cwd: Option<PathBuf>, last: Option<String>) -> Option<String>` — the folder to open, from the first argument, the working folder and the repository opened last, in that order.
- `pub fn repo() -> Option<String>` — the folder to open at startup, if any.
- `pub fn remember(path: &str) -> Vec<String>` — remember `path` as the repository to open next time, and put it first in the recent list. Returns the recent list.
- `pub fn recent() -> Vec<String>` — the folders of the repositories opened in the app, newest first.
- `pub fn keep_recent(registered: &[&str]) -> Vec<String>` — drop from the recent list every folder that is not `registered`, and return what is left. The setting is written only when it changes.
- `fn parse(text: &str) -> Vec<String>` — the recent list in the setting's text: one folder per line.
- `fn touched(mut list: Vec<String>, path: &str) -> Vec<String>` — `list` with `path` moved, or added, to the front.
- `fn pruned(list: &[String], registered: &[&str]) -> Vec<String>` — the folders of `list` that are `registered`, in the same order.

## Depends on

- [[cw-core]] — uses `MANIFEST_DIR`, `VAULT_DIR`
- [[Prefs Module]] — uses `read`, `write`

## Used by

- [[codewiki-app]] — from `connect`, `listen`, `main`, `new`
- [[Instance Module]] — from `acquire`, `new`

## See also

- [[_index]]
- [[codewiki-app]]
