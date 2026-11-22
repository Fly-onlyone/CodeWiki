---
tags: [core, cw-core, module]
---

# Agents Off Module

> The list of folders closed to coding agents.

## What it does

One JSON file in the data directory holds the list. The MCP bridge, the session-start hook, the command line and the desktop app all read and change it through this module.

## Source

- `crates/cw-core/src/agents_off.rs` — module `cw_core::agents_off`

## How it works

1. `agents` and `session_start_hook` in [[codewiki]], and 5 more, call `blocking`. `blocking` calls `covering`, then `forms`. `forms` calls `components`, then `strip_verbatim`.
2. `agents` in [[codewiki]] and `listed_folders` in [[codewiki-mcp]], and 2 more, call `unreadable`.
3. `agents` in [[codewiki]] and `Bridge::screen` in [[codewiki-mcp]], and 2 more, call `file`.
4. `agents` and `session_start_hook` in [[codewiki]], and 3 more, call `load`. `load` calls `file`, then `Error::invalid` in [[Error]].
5. `agents` in [[codewiki]] and `kept_by` in [[Live Module]], and 3 more, call `normalize`.

6 more entry points: `covering`, `add`, `remove`, `refusal`, `repo_refusal`, `reset`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const FILE: &str` — name of the list file inside the data directory.
- `const VERSION: u32` — the only layout of the file this version understands.
- `pub enum Added` — what `add` did.
- `pub enum Removed` — what `remove` did.
- `pub fn file(data_dir: &Path) -> PathBuf` — path of the list file.
- `pub fn load(data_dir: &Path) -> Result<Vec<PathBuf>>` — the listed folders. A missing file is an empty list; a file that cannot be read or understood is an error, and so is an entry that is empty or not absolute: `""` would cover every folder and `"secret"` would mean a different folder to each reader.
- `pub fn normalize(path: &Path, base: &Path) -> PathBuf` — make `path` absolute (relative paths are joined to `base`), resolve `.` and `..` without touching the disk, and drop any trailing separator. A path that joining leaves relative (on Windows a drive without a root, `D:secret`) is resolved by the system first, against the process's folder on that drive. Only when the system cannot do that, for a path with a NUL in it or one too long for it, is the result still relative; the backend cannot open such a path either, and `load` and `add` refuse it.
- `pub fn covering(entries: &[PathBuf], path: &Path) -> Option<PathBuf>` — the listed entry that is `path` or a parent of it, if any. Both sides are compared as they are on disk (links followed) when they exist, and as written otherwise. Windows ignores case. `path` must be absolute.
- `pub enum Block` — why coding agents get no CodeWiki in a folder.
- `pub fn blocking(entries: &[PathBuf], path: &Path) -> Option<Block>` — whether `entries` keep coding agents out of `path`: an entry that covers it comes first, then one inside it. Compared as `covering` compares, so `D:\work\client` neither holds `D:\work\clients\x` nor lies in it. `path` must be absolute.
- `pub fn refusal(entry: &Path) -> String` — what an agent is told when a folder of its session is kept from coding agents: `entry` is the listed folder that keeps it out.
- `pub fn repo_refusal() -> &'static str` — what an agent is told when only the repository a tool names is kept from coding agents. It names no path, so it tells nothing about the folder.
- `pub fn unreadable(file: &Path) -> String` — what the MCP bridge, the command line and the app say when the list (`file`) cannot be read.
- `pub fn add(data_dir: &Path, folder: &Path) -> Result<Added>` — add `folder` to the list unless a listed folder already covers it.
- `pub fn remove(data_dir: &Path, folder: &Path) -> Result<Removed>` — take `folder` off the list. A folder that only a listed parent covers is left alone.
- `pub fn reset(data_dir: &Path) -> Result<()>` — write an empty list over whatever the file holds, even a broken file.
- `fn forms(path: &Path) -> Vec<Vec<String>>` — the component lists a path can be compared by: as written (normalized), and as the disk has it when the path exists and that differs.
- `fn components(path: &Path) -> Vec<String>` — private function, returns `Vec<String>`; called by `forms`.
- `fn strip_verbatim(path: &Path) -> PathBuf` — drop the `\\?\` prefix `canonicalize` adds on Windows.

And 4 more private items with no comment.

## Depends on

- [[Error]] — uses `external`, `invalid`, `io`
- [[Error Module]] — uses `Result`

## Used by

- [[Guard Module]] — from `direct_path`, `hide_repos`, `repo_arg`, `session_folders`
- [[codewiki]] — from `a_broken_list_is_reported_and_left_alone`, `agents`, `session_start_hook`, `skills_stop_at_the_agents_refusal`
- [[codewiki-app]] — from `keep_agents_out`, `let_agents_in`, `run_settings_action`
- [[codewiki-mcp]] — from `call`, `listed_folders`, `screen`
- [[Live Module]] — from `kept_by`
- [[Settings View Module]] — from `read_agents_off`

## See also

- [[_index]]
- [[cw-core]]
