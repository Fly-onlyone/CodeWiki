---
tags: [core, cw-vault, module]
---

# Manifest Module

> Machine metadata stored in `obsidian/.codewiki/`.

## What it does

Defines and stores the bookkeeping CodeWiki keeps about a vault. Notes carry only a `tags` frontmatter, so everything else the generator must remember sits in a folder of plain files under the vault: which source files a note cites and their hashes, the hash of the note as written, whether a person or an agent touched it, and which proposals were rejected. [[VaultMeta]] loads and saves it all. The files are sorted text, so they diff and merge cleanly in git.

## Source

- `crates/cw-vault/src/manifest.rs` — module `cw_vault::manifest`

## How it works

Three files are written under the metadata folder:

- `manifest.json` holds one `Manifest`: schema number, template pack and version, and the last commit the vault was brought up to date with.
- `notes.jsonl` holds one JSON line per [[NoteEntry]], sorted by note id. Empty fields are left out.
- `registry.json` is derived from the entries on every save, so it can never disagree with them. It feeds the [[Registry]] that resolves links.

`VaultMeta::load` reads these. A missing file is not an error: a vault with no metadata loads as the empty default. A line of `notes.jsonl` that cannot be parsed is an error that names the line number.

Each entry has an `Origin` (`Generated` or `Adopted`), its [[SourceRef]] list, a `body_hash` and an `input_hash`. `body_hash` hashes the note text after normalizing it, so line-ending changes do not count as edits. `Explained` records what an agent wrote and the files and hashes it wrote from. `behind` holds the reasons an explanation may be out of date. [[Lifecycle Module]] reads these fields to decide what may be rewritten.

## Items

- `pub const MANIFEST_SCHEMA: u32` — public constant; used by `Manifest::default`.
- `pub struct Manifest` — public struct; used by `VaultMeta`.
- `pub enum Origin` — public enum; used by `NoteEntry`, `NoteInfo`.
- [[SourceRef]] — A source file a note describes, with the content hash seen at generation.
- `pub struct Explained` — what a coding agent explained a note from.
- [[Gone]] — why a note is still in the vault although what it describes is gone from the repository.
- [[NoteEntry]] — struct
- `pub fn body_hash(text: &str) -> String` — hash used to tell whether a note changed on disk.
- [[VaultMeta]] — everything under `.codewiki/`, loaded.

## Depends on

- [[Hash Module]] — uses `normalize_text`, `sha256_hex`
- [[Error Module]] — uses `Result`
- [[SourceRef]] — uses `SourceRef`

## Used by

- [[cw-wiki]] — from `a_brief_says_which_sections_are_the_agents`, `a_code_change_refreshes_the_facts_and_keeps_the_explanation`, `a_note_its_owner_edited_gets_a_proposal_and_is_not_written`, `a_submission_is_woven_into_the_note_and_recorded`
- [[Proposal Module]] — from `Offer`, `Proposal`, `approve`, `create_as`
- [[Explanations Module]] — from `brief`, `record`, `submit`, `untouched`
- [[Generate Module]] — from `adopt_existing`, `carry_moves`, `generate_with`
- [[VaultMeta]] — from `VaultMeta`, `load`, `save`
- [[Gone Module]] — from `after_removal`, `retire`
- [[Lifecycle Module]] — from `state`
- [[Query Module]] — from `NoteInfo`

## Gotchas

- Loading and saving rewrite the whole file set each time. Save writes `registry.json` too, even though nobody edits it by hand.
- `body_hash` ignores CRLF versus LF. A note saved with Windows line endings is not seen as edited.
- `rejected_inputs` is permanent memory: an input a person rejected is never proposed again.
- Setting `explained` changes how the note is rewritten later. Rules then write only the sections the agent did not write.
- A broken line in `notes.jsonl` stops the whole load, so a bad merge in git must be fixed by hand.

## See also

- [[_index]]
- [[cw-vault]]
