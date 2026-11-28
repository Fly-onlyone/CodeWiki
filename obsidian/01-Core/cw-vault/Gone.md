---
tags: [core, cw-vault]
---

# Gone

> Why a note is still in the vault although what it describes is gone from the repository.

## What it does

When the code a note describes is deleted, the note does not always leave the vault at once. A note that holds written work could lose that work. `Gone` is the reason a note was kept. The manifest stores it on the note's entry, so the next sync and the app both know why the note is still there.

## Source

- `crates/cw-vault/src/manifest.rs` — definition of `Gone`

## How it works

The four values:

- `Pending`: the note waits in a retire proposal for its owner's decision.
- `History`: the owner chose to keep it as history. It is never proposed again.
- `Uncommitted`: the deletion of what it describes is not committed yet. A deleted file may still come back.
- `Unsaved`: git could not bring the note back, because it is untracked or changed since its last commit.

The verdict functions in [[Gone Module]] choose a value. Approving a retire proposal deletes a note only if it is `Pending`. Rejecting a proposal, or listing a note under keep, turns it into `History`.

## Items

Declared as `pub enum Gone`.

## Used by

- [[Generate Module]] — from `KeptGone`, `generate_with`
- [[Gone Module]] — from `Hand`, `Verdict`
- [[cw-wiki]] — from `kept_gone`
- [[Query Module]] — from `NoteInfo`
- [[NoteEntry]] — from `NoteEntry`

## Gotchas

- `History` is final: a note with it is not judged again and its links are left alone.
- `Uncommitted` and `Unsaved` are not decisions. A later sync can change them once the deletion is committed or the note is saved in git.

## See also

- [[_index]]
- [[Manifest Module]]
