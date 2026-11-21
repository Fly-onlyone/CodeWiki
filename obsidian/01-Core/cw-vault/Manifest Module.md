---
tags: [core, cw-vault, module]
---

# Manifest Module

> Machine metadata stored in `obsidian/.codewiki/`.

## What it does

Notes keep the owner's frontmatter convention (tags only), so everything the generator needs to track a note lives here instead. The files are plain text and sorted, so they diff and merge cleanly in git.

## Source

- `crates/cw-vault/src/manifest.rs` — module `cw_vault::manifest`

## How it works

1. `state` in [[Lifecycle Module]] and `approve` in [[Proposal Module]], and 7 more, call `body_hash`. `body_hash` calls `sha256_hex`, then `normalize_text` (both in [[Hash Module]]).
2. `VaultMeta::load` in [[VaultMeta]] calls `Manifest::default`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const MANIFEST_SCHEMA: u32` — public constant; used by `Manifest::default`.
- `pub struct Manifest` — public struct; used by `VaultMeta`.
- `pub enum Origin` — public enum; used by `NoteEntry`, `NoteInfo`.
- [[SourceRef]] — A source file a note describes, with the content hash seen at generation.
- `pub struct Explained` — what a coding agent explained a note from.
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
- [[VaultMeta]] — from `VaultMeta`, `load`, `save`
- [[Generate Module]] — from `adopt_existing`, `generate_with`
- [[Lifecycle Module]] — from `state`
- [[Query Module]] — from `NoteInfo`
- [[NoteEntry]] — from `NoteEntry`

## See also

- [[_index]]
- [[cw-vault]]
