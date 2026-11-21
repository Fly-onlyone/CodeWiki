---
tags: [core, cw-vault, module]
---

# Proposal Module

> The review queue: proposed changes to human-edited notes.

## What it does

A proposal stores the full proposed text plus a unified diff for reading. Nothing in the vault changes until a human approves it, and approval only applies when the note is still the version the proposal was written against.

Two things propose. A sync does when the code under a note changed. A coding agent does when it explained a note its owner had edited: its text and pictures wait here instead of replacing the owner's words.

## Source

- `crates/cw-vault/src/proposal.rs` — module `cw_vault::proposal`

## How it works

1. `run` in [[codewiki]] and `list_proposals` in [[Api Module]], and 3 more, call `list`. `list` calls `proposals_dir`, then `Error::io` in [[Error]], then `read_json`. `proposals_dir` calls `VaultMeta::dir` in [[VaultMeta]]. `read_json` calls `Error::io`, then `Error::external` (both in [[Error]]).
2. `get_proposal` in [[Api Module]] and `approve` in [[Explanations Module]] call `proposed_text`. `proposed_text` calls `check_id`, then `proposals_dir`, then `Error::io` in [[Error]]. `check_id` calls `Error::invalid` in [[Error]].
3. `get_proposal` in [[Api Module]] and `approve` in [[Explanations Module]] call `pictures_text`. `pictures_text` calls `check_id`, then `proposals_dir`, then `Error::io` in [[Error]].
4. `run` in [[codewiki]] and `approve` in [[Explanations Module]] call `approve`. `approve` calls `get`, then `Error::invalid`, then `Error::io` (both in [[Error]]), then `body_hash` in [[Manifest Module]], then `write_json`, then `proposed_text`. `write_json` calls `proposals_dir`, then `Error::external`, then `Error::io` (both in [[Error]]).
5. `run` in [[codewiki]] and `reject` in [[Explanations Module]] call `reject`. `reject` calls `get`, then `Error::invalid` in [[Error]], then `write_json`.

4 more entry points: `create`, `create_agent`, `get`, `patch_text`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub enum ProposalStatus` — public enum; used by `Proposal`.
- `pub enum ProposalKind` — what made a proposal.
- `pub struct Offer` — what an agent's proposal carries beside the note's text.
- `pub struct Proposal` — public struct; used by `ProposalDetail`, `approve`, `approve_proposal`, `create` and 9 more.
- `fn proposals_dir(vault: &Path) -> PathBuf` — private function, returns `PathBuf`; called by `create_as`, `get`, `list`, `patch_text` and 4 more.
- `pub fn unified_diff(old: &str, new: &str, path: &str) -> String` — unified diff between two note texts, for display.
- `fn read_json(path: &Path) -> Result<Proposal>` — private function, returns `Result<Proposal>`; called by `get`, `list`.
- `fn write_json(vault: &Path, proposal: &Proposal) -> Result<()>` — private function, returns `Result<()>`; called by `approve`, `create_as`, `reject`.
- `create` — queue a sync's proposal. `current` is the note text on disk, empty for a new note. An earlier pending proposal of a sync for the same note is replaced; one of an agent stays.
- `create_agent` — queue an agent's proposal: the note with the agent's sections woven in, and what comes with it. An earlier pending proposal of an agent for the same note is replaced; one of a sync stays, since it says something else: that the facts changed.
- `pub fn list(vault: &Path) -> Result<Vec<Proposal>>` — all proposals, sorted by id.
- `pub fn get(vault: &Path, id: &str) -> Result<Proposal>` — public function, returns `Result<Proposal>`; called by `approve`, `get_proposal`, `reject`.
- `pub fn proposed_text(vault: &Path, id: &str) -> Result<String>` — proposed full text of the note.
- `pub fn patch_text(vault: &Path, id: &str) -> Result<String>` — public function, returns `Result<String>`; called by `get_proposal`.
- `pub fn pictures_text(vault: &Path, id: &str) -> Result<String>` — the pictures file an agent's proposal would give the note; empty when it carries none.
- `const PICTURES: &str` — extension of the file that holds a proposal's pictures.
- `fn slug(note_id: &str) -> String` — File-name-safe form of a note id such as `type:cw_core::Error`.
- `fn check_id(id: &str) -> Result<()>` — ids come from API callers; never let one name a path outside the folder.
- `pub fn approve(vault: &Path, id: &str) -> Result<Proposal>` — apply a pending proposal: write the note and record the outcome. Fails with the proposal marked `Superseded` when the note changed meanwhile.
- `pub fn reject(vault: &Path, id: &str) -> Result<Proposal>` — reject a pending proposal. The caller records `input_hash` on the note entry so the same change is not proposed again.

And 2 more private items with no comment.

## Depends on

- [[Error]] — uses `external`, `invalid`, `io`, `not_found`
- [[Manifest Module]] — uses `Explained`, `body_hash`
- [[Error Module]] — uses `Result`
- [[Hash Module]] — uses `sha256_hex`
- [[VaultMeta]] — uses `dir`

## Used by

- [[cw-wiki]] — from `a_fault_the_owner_wrote_is_not_held_against_the_agent`, `a_note_its_owner_edited_gets_a_proposal_and_is_not_written`, `an_explained_note_changed_by_hand_gets_a_proposal_with_the_explanation`, `human_edited_notes_get_a_proposal_not_an_overwrite`
- [[Api Module]] — from `ProposalDetail`, `approve_proposal`, `get_proposal`, `list_proposals`
- [[Explanations Module]] — from `approve`, `reject`, `submit`
- [[codewiki]] — from `run`
- [[Briefing Module]] — from `session_start`
- [[Generate Module]] — from `generate_with`
- [[Query Module]] — from `status`

## See also

- [[_index]]
- [[cw-vault]]
