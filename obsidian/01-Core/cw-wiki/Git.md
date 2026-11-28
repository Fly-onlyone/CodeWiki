---
tags: [core, cw-wiki]
---

# Git

> What git knows about a repository, each answer asked for once and only when a note needs it: most syncs remove nothing.

## What it does

A lazy view of what git knows about the repository, used when a sync decides about notes whose subject is gone. Each question (which files are recoverable, which changed, what is at HEAD, the full history, which files moved) runs a git command at most once and only when a note needs it. Most syncs remove nothing, so most syncs never run them.

## Source

- `crates/cw-wiki/src/gone.rs` — definition of `Git`

## How it works

Each answer sits in a `OnceCell`, filled by a function of the `git` module on the first call. `new` is cheap and builds nothing. `knew` says a commit once held a path, as a file or, from a folder set built from the history, as a folder. `has_at_head` says the commit at HEAD still has the path, so the deletion is not committed. `holds` says git can bring a note back as it is now: the note file must be in the recoverable set, and so must its picture, if it has one. `of_generated`, `of_adopted` and `cited` in [[Gone Module]] use these answers.

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

## Gotchas

- `changed` and `head` are `Option`. They are `None` when git cannot answer, for example no repository. Then `has_at_head` is false.
- `holds` checks the note file and its picture. A note with a picture that git does not hold counts as unsaved.

## See also

- [[_index]]
- [[Gone Module]]
