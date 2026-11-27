---
tags: [core, cw-wiki]
---

# Git

> What git knows about a repository, each answer asked for once and only when a note needs it: most syncs remove nothing.

## Source

- `crates/cw-wiki/src/gone.rs` — definition of `Git`

## How it works

1. `generate_with` in [[Generate Module]] and `retire` in [[Gone Module]] call `new`.

## Items

Declared as `pub(crate) struct Git<'a>`.

Members:

- `pub(crate) fn new(root: &'a Path) -> Self` — internal method, returns `Self`; called by `generate_with`, `retire`.
- `pub(crate) fn root(&self) -> &Path` — the repository folder this view is of.
- `pub(crate) fn knew(&self, path: &str) -> bool` — true when a commit once held `path`, as a file or as a folder.
- `pub(crate) fn has_at_head(&self, path: &str) -> bool` — true when the commit at `HEAD` still has `path`, as a file or as a folder: its deletion is not committed.
- `pub(crate) fn recoverable(&self) -> &HashSet<String>` — see `git::recoverable`.
- `pub(crate) fn changed(&self) -> Option<&HashSet<String>>` — see `git::changed`.
- `pub(crate) fn head(&self) -> Option<&HashSet<String>>` — see `git::at_head`.
- `pub(crate) fn history(&self) -> &HashSet<String>` — see `git::history_paths`.
- `pub(crate) fn moves(&self) -> &HashMap<String, String>` — see `git::moves`.
- `pub(crate) fn holds(&self, entry: &NoteEntry) -> bool` — true when git holds the note of `entry`, and its pictures, as they are on disk now.

## Depends on

- [[Git Module]] — uses `at_head`, `changed`, `history_paths`, `moves`
- [[Generate Module]] — uses `illustration_path`
- [[NoteEntry]] — uses `NoteEntry`

## Used by

- [[Gone Module]] — from `after_removal`, `cited`, `of_adopted`, `of_generated`
- [[Generate Module]] — from `carry_moves`, `generate_with`

## See also

- [[_index]]
- [[Gone Module]]
