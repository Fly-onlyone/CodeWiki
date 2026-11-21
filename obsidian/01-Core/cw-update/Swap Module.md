---
tags: [core, cw-update, module]
---

# Swap Module

> Replacing the files of an installation, and undoing it.

## What it does

Every step is a rename inside the installation folder. Before each one the journal `.update\swap.json` records it, so that a swap cut short by a crash or a power cut is undone the next time the app starts.

## Source

- `crates/cw-update/src/swap.rs` — module `cw_update::swap`

## How it works

1. `install` in [[cw-update]] calls `swap`. `swap` calls `old_name`, then `write_journal`, then `rename_patiently`, then `undo`, then `journal_path`, then `close_journal`. `old_name` calls `remove`. `write_journal` calls `journal_path`.
2. `AppView::start_updates` in [[codewiki-app]] calls `recover_and_clean`. `recover_and_clean` calls `journal_path`, then `undo`, then `replaced_name`, then `remove`, then `rename_patiently`. `undo` calls `rename_patiently`.
3. `AppView::start_updates` in [[codewiki-app]] calls `leftovers`. `leftovers` calls `replaced_name`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const NOT_INSTALLED: [&str; 2]` — files of a package that are not installed: the installer itself.
- `const ATTEMPTS: u32` — A file that was just written, or a program that just ended, can be held by Windows Defender or the search indexer for a moment. Try this many times, this long apart (as `uninstall.ps1` does).
- `fn journal_path(install: &Path) -> PathBuf` — private function, returns `PathBuf`; called by `close_journal`, `recover_and_clean`, `swap`, `write_journal`.
- `fn write_journal(install: &Path, journal: &Journal) -> Result<(), String>` — private function, returns `Result<(), String>`; called by `close_journal`, `swap`.
- `fn rename_patiently(from: &Path, to: &Path) -> std::io::Result<()>` — private function, returns `std::io::Result<()>`; called by `recover_and_clean`, `swap`, `undo`.
- `fn close_journal(install: &Path)` — mark the swap as finished: delete its journal. While the journal exists, the next start undoes the moves it lists, so one that cannot be deleted (a scanner may hold a new file for a moment) is emptied instead, and tried again for a while before giving up.
- `const OLD_CUDA_DIR: &str` — the folder and program of the removed graphics-card embedding worker.
- `fn remove(path: &Path) -> std::io::Result<()>` — private function, returns `std::io::Result<()>`; called by `old_name`, `recover_and_clean`.
- `fn old_name(install: &Path, name: &str) -> Result<PathBuf, String>` — `<name>.old`, or `.old2`, `.old3`… when an older one is still in use (a program from two updates ago that has not ended).
- `fn undo(moves: &[Move]) -> Vec<String>` — undo `moves` from the last to the first. A move whose target is gone or whose source is back already is skipped, so this can run again after a crash. Returns the moves that could not be undone.
- `pub fn swap(install: &Path, stage: &Path) -> Result<(), String>` — move every entry of `stage` into `install`, renaming what it replaces to `<name>.old`. On the first failure every step is undone and the installation is as it was.
- `pub fn recover_and_clean(install: &Path) -> Vec<String>` — run when the app starts. Undo a swap that was cut short, then delete the `.old` files left by a finished one and the `.update` folder. A `.old` file whose replacement is missing is put back instead. Returns what it did, for the log; files still in use are left for the next start.
- `pub fn leftovers(install: &Path) -> Vec<String>` — what `recover_and_clean` has still to delete: the `.old` entries and the `.update` folder, by name. A program that is still running cannot be deleted, so right after an update the app it replaced is usually left.
- `fn replaced_name(file_name: &str) -> Option<&str>` — for `codewiki.exe.old` or `codewiki.exe.old2`, the name it replaced.

And 5 more private items with no comment.

## Depends on

- [[cw-update]] — uses `UPDATE_DIR`

## Used by

- [[codewiki-app]] — from `start_updates`
- [[cw-update]] — from `install`

## See also

- [[_index]]
- [[cw-update]]
