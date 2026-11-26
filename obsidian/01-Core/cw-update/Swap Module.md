---
tags: [core, cw-update, module]
---

# Swap Module

> Replacing the files of an installation, and undoing it.

## What it does

Replaces the files of a running installation with the files of a new, unpacked package, and can undo that if anything goes wrong. Windows lets a running program be renamed but not overwritten, so every change is a rename inside the installation folder. A journal file, `.update\swap.json`, records each rename before it is made. If the app crashes or the power fails mid-swap, the next start reads the journal and puts everything back.

## Source

- `crates/cw-update/src/swap.rs` — module `cw_update::swap`

## How it works

`swap` lists the entries of the staged package (minus `install.ps1` and `install.cmd`) and goes through them in name order. For an entry that already exists in the installation, it first plans a move to `<name>.old` (or `.old2`, `.old3` when an older copy is still in use, picked by `old_name`). Then it plans a move of the new entry into place. Each move is appended to the journal and written to disk, and only then made with `rename_patiently`. On the first failure `undo` reverses the moves from last to first. If that works, the journal is deleted and the error ends with "nothing was changed". If some move cannot be undone, the journal stays and the error says the next start will try again. On success `close_journal` deletes the journal.

`recover_and_clean` runs when the app starts. It undoes an unfinished journal, then walks the installation: a `.old` entry whose current file exists is deleted, one without it is renamed back. It deletes the `.update` folder and the old graphics-card worker folder `cuda`. `leftovers` lists what is still to delete, for the UI.

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

## Gotchas

- A step is recorded before it is made. `undo` skips a step whose target does not exist or whose source is back, so a recorded but unmade step is harmless and `undo` can run again after a crash.
- `rename_patiently` tries 20 times, 250 ms apart, because Defender or the search indexer may hold a file for a moment. A swap can therefore wait several seconds before it fails.
- If the journal cannot be deleted, `close_journal` empties it instead. An empty journal means "finished", so `recover_and_clean` only removes it. A non-empty journal would roll back a good update.
- Right after an update the `.old` files of running programs cannot be deleted. They stay until the next start.
- The `cuda` folder is removed only if it holds `codewiki-embed-cuda.exe`. A folder of that name without the worker is left alone.
- A file held open with no sharing makes the whole swap fail and roll back. The staged package stays for another try.

## See also

- [[_index]]
- [[cw-update]]
