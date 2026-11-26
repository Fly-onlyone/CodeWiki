---
tags: [core, cw-vault, module]
---

# Lifecycle Module

> Note lifecycle: what may be done to a note given its state on disk.

## What it does

Decides what the updater may do to a note, based on the note's state on disk. CodeWiki rewrites notes when code changes, but a note that a person edited must not be overwritten. This module holds that rule in two small functions: `state` classifies a note, and `action` turns the state into a decision.

## Source

- `crates/cw-vault/src/lifecycle.rs` — module `cw_vault::lifecycle`

## How it works

`state` takes the manifest entry ([[NoteEntry]]), the current file text (if any) and a flag saying whether a cited source changed.

- No file: `Missing`.
- A note counts as edited when its origin is `Adopted`, or when the hash of its body differs from the hash CodeWiki stored (`body_hash` in [[Manifest Module]]).
- Edited or not, crossed with source changed or not, gives `Generated`, `Stale`, `HumanEdited` or `HumanEditedStale`.

`action` takes the entry, the state and the hash of the inputs a rewrite would use now:

- `Missing` gives `Regenerate`.
- `Generated` or `Stale`: if the input hash is unchanged, `Keep`. Otherwise, if an agent explained the note, `Merge`: write the rule-made sections and keep the agent's. Otherwise `Regenerate`.
- `HumanEdited` gives `Keep`.
- `HumanEditedStale` gives `Keep` if the inputs are unchanged or were rejected before. Otherwise `Propose`: the new text waits as a proposal for a person to review.

## Items

- `pub enum NoteState` — public enum; used by `NoteInfo`, `action`, `state`.
- `pub enum Action` — what the updater should do with a note.
- `pub fn state(entry: &NoteEntry, disk: Option<&str>, sources_changed: bool) -> NoteState` — classify a note. `disk` is the current file text, if the file exists. `sources_changed` is true when any cited source hash differs from the manifest.
- `pub fn action(entry: &NoteEntry, state: NoteState, new_input_hash: &str) -> Action` — decide the action. `new_input_hash` is the hash of the inputs a regeneration would use now.

## Depends on

- [[Manifest Module]] — uses `body_hash`
- [[NoteEntry]] — uses `NoteEntry`

## Used by

- [[Query Module]] — from `NoteInfo`, `info`
- [[Generate Module]] — from `generate_with`
- [[Health Module]] — from `doc_drift`

## Gotchas

- An adopted note counts as human-edited even if untouched since adoption. It is never regenerated.
- Line endings do not make a note edited. The test shows a body with CRLF line endings still counts as `Generated`.
- A human-edited note is never given `Regenerate`. A test checks this for every state and input.
- `rejected_inputs` remembers inputs whose proposals a person turned down, so the same proposal does not come back.
- `Merge` applies only to notes that were not touched by hand after an agent explained them. A hand edit after that turns the case into `Propose`.

## See also

- [[_index]]
- [[cw-vault]]
