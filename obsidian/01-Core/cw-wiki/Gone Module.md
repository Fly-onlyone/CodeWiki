---
tags: [core, cw-wiki, module]
---

# Gone Module

> Notes whose subject is gone from the repository: which of them leave the vault in a sync, which stay, and why.

## What it does

A note nobody wrote in is the rules' alone and goes with its subject at once. A note that holds written work, an agent's explanation or its owner's edits, goes only when nothing is lost by it: the deletion of its subject is committed, and git holds the note as it is now.

## Source

- `crates/cw-wiki/src/gone.rs` — module `cw_wiki::gone`

## How it works

1. `generate_with` in [[Generate Module]] calls `of_generated`.
2. `generate_with` in [[Generate Module]] calls `cited`. `cited` calls `spans`, then `path_in`.
3. `generate_with` in [[Generate Module]] calls `repath`. `repath` calls `spans`, then `path_in`.
4. `generate_with` in [[Generate Module]] calls `of_adopted`.
5. `generate_with` in [[Generate Module]] calls `after_removal`. `after_removal` calls `Note::parse` in [[Note Module]], then `body_hash` in [[Manifest Module]], then `unlink_dead`, then `extract`, then `weave` (all in [[Explanation Module]]), then `Error::io` in [[Error]].

3 more entry points: `remove_empty_folders`, `retire`, `keep_all`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- [[Git]] — what git knows about a repository, each answer asked for once and only when a note needs it: most syncs remove nothing.
- `pub(crate) enum Verdict` — what a sync does with a note whose subject is gone.
- `pub(crate) fn of_generated(entry: &NoteEntry, git: &Git) -> Verdict` — the verdict for a note CodeWiki wrote that holds written work and has no place in the plan any more.
- `fn path_in(span: &str) -> Option<String>` — the repository path a code span of a note names, if it has the shape of one: `src/a.rs`, src/a.rs:12, `./src/`, `src\a.rs`. Whether such a path ever existed is the caller's question.
- `fn spans(text: &str) -> impl Iterator<Item = (bool, &str)>` — the code spans of a note, each with whether it stands inside backticks.
- `pub(crate) struct Cited` — the repository paths a note names in its text, by what became of them. A span that never named a path of the repository is in none of them.
- `pub(crate) fn cited(text: &str, git: &Git) -> Cited` — read the paths the note `text` names anywhere in it.
- `pub(crate) fn repath(text: &str, moved: &[(String, String)]) -> String` — the note `text` with each moved path it names replaced by where the file is now. Only code spans change, and only the path in them.
- `pub(crate) enum Hand` — what a sync finds about a note its owner wrote.
- `pub(crate) fn of_adopted(entry: &NoteEntry, cited: &Cited, git: &Git) -> Hand` — the verdict for a note its owner wrote, from the paths it names.
- `pub(crate) const UNLINKED: &str` — how a line of `NoteEntry::behind` starts that says a link of the agent's became plain words because its note was removed.
- `pub(crate) struct Left` — A note that left the vault, with its text as it was.
- `pub(crate) struct Cleanup` — what `after_removal` changed beside the notes that left.
- `const ATTACHMENTS: &str` — folder of a vault for the files notes show. CodeWiki writes no note there and judges none.
- `after_removal` — tidy the vault after the notes `left` were deleted from it and from `meta`, which the caller saves. A link to one of them becomes plain words in every note that stays. In a note an agent explained and nobody edited, that is the agent's text changing, so the note is marked behind. A note its owner wrote or edited is changed only when git holds it as it is. Only links to the notes that just left are touched: any other link that leads nowhere is its owner's to mend. An attachment that only removed notes showed is deleted, when git holds it.
- `pub(crate) fn remove_empty_folders(vault: &Path) -> Result<()>` — remove every folder of the vault that holds nothing, deepest first: a folder whose notes all left is no part of the wiki. Folders that are not for notes are left alone, empty or not.
- `fn tidy(path: &str) -> String` — A vault path as the manifest writes it.
- `pub(crate) fn retire(root: &Path, id: &str, keep: &[String]) -> Result<Proposal>` — approve the retire proposal `id` of the repository at `root`: delete the notes it names, except those in `keep` (vault paths), which stay as history. A note is deleted only if it is still the note the proposal saw, still marked as waiting for this decision, and held by git as it is. One that fails any of these stays and is judged again by the next sync.
- `pub(crate) fn keep_all(root: &Path, id: &str) -> Result<Proposal>` — reject the retire proposal `id`: every note it names stays, as history, and is not proposed again.

And 1 more private item with no comment.

## Depends on

- [[Git]] — uses `Git`, `changed`, `has_at_head`, `holds`
- [[VaultMeta]] — uses `VaultMeta`, `load`, `registry`, `save`
- [[Explanation Module]] — uses `extract`, `unlink_dead`, `weave`
- [[Proposal Module]] — uses `Proposal`, `close_retire`, `get`
- [[Error]] — uses `invalid`, `io`
- [[cw-core]] — uses `VAULT_DIR`
- [[Error Module]] — uses `Result`
- [[Manifest Module]] — uses `body_hash`

## Used by

- [[Explanations Module]] — from `approve_keeping`, `reject`
- [[Generate Module]] — from `generate_with`

## See also

- [[_index]]
- [[cw-wiki]]
