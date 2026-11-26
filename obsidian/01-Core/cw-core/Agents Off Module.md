---
tags: [core, cw-core, module]
---

# Agents Off Module

> The list of folders closed to coding agents.

## What it does

Lets a user close chosen folders to coding agents. In a closed folder, CodeWiki's session-start hook and MCP tools stay silent. One JSON file in the data directory (`agents-off.json`) holds the list. The MCP bridge, the hook, the command line (`codewiki agents`) and the desktop app all read and change it through this module, so they agree on what is closed.

## Source

- `crates/cw-core/src/agents_off.rs` — module `cw_core::agents_off`

## How it works

`load` reads the file. A missing file is an empty list. A file that cannot be read, parsed, has another version number, or holds a non-absolute entry is an error.

Matching is done on path components, not text. `forms` gives up to two spellings of a path: as written after `normalize`, and as the disk has it (links followed, the Windows `\\?\` prefix removed by `strip_verbatim`). Components are lower-cased on Windows. `covering` finds an entry that equals the path or is its parent. `blocking` adds the other direction: an entry lying inside the folder gives `Block::Contains`, because indexing the folder would index the entry's code. A covering entry is reported first as `Block::Covered`.

`add` skips a folder already covered and returns `Added::CoveredBy`. `remove` takes off only an exact entry; a folder covered only by a listed parent returns `Removed::CoveredBy` and nothing is written. `save` writes a temporary file and renames it, so a reader never sees half a list. `reset` overwrites even a broken file.

`refusal`, `repo_refusal` and `unreadable` build the fixed messages an agent or user sees. The refusal text tells the agent to stop calling CodeWiki tools.

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

## Gotchas

- Fail closed: callers treat a list that cannot be read as "tools off", never as "nothing listed". `reset` is the way out.
- An empty entry would cover every folder, and a relative one would mean different folders to different readers, so `load` rejects both.
- `D:\work\client` does not cover `D:\work\clients\x`; comparison is by whole components.
- `normalize` does not touch the disk, but `covering` does call `canonicalize` for existing paths. A symlink to a closed folder is closed too.
- `covering` and `blocking` need an absolute path.
- Removing a subfolder of a closed parent does not open it. Remove the parent entry.
- `refusal` and `repo_refusal` texts are asserted exactly in tests. Change them with care: agents read them.

## See also

- [[_index]]
- [[cw-core]]
