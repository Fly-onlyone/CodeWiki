---
tags: [core, cw-update, module]
---

# Swap Module

> Replacing the files of an installation, and undoing it.

## Source

- `crates/cw-update/src/swap.rs` — module `cw_update::swap`

## How it works

Main items:

- `swap` — move every entry of `stage` into `install`, renaming what it replaces to `<name>.old`.
- `leftovers` — what `recover_and_clean` has still to delete: the `.old` entries and the `.update` folder, by name.
- `recover_and_clean` — run when the app starts.
- `Move` — struct
- `journal_path` — function
- `remove` — function
- `rename_patiently` — function
- `JOURNAL` — const

## Depends on

- [[cw-update]] — uses `UPDATE_DIR`

## Used by

- [[codewiki-app]] — from `start_updates`
- [[cw-update]] — from `install`

## See also

- [[_index]]
- [[cw-update]]
