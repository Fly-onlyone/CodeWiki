---
tags: [core, cw-vault, module]
---

# Proposal Module

> The review queue: proposed changes to human-edited notes.

## What it does

A proposal stores the full proposed text plus a unified diff for reading. Nothing in the vault changes until a human approves it, and approval only applies when the note is still the version the proposal was written against.

## Source

- `crates/cw-vault/src/proposal.rs` — module `cw_vault::proposal`

## How it works

1. `run` in [[codewiki]] and `list_proposals` in [[Api Module]], and 3 more, call `list`. `list` calls `proposals_dir`, then `Error::io` in [[Error]], then `read_json`. `proposals_dir` calls `VaultMeta::dir` in [[VaultMeta]]. `read_json` calls `Error::io`, then `Error::external` (both in [[Error]]).
2. `run` in [[codewiki]] and `approve_proposal` in [[Api Module]] call `approve`. `approve` calls `get`, then `Error::invalid`, then `Error::io` (both in [[Error]]), then `body_hash` in [[Manifest Module]], then `write_json`, then `proposed_text`. `write_json` calls `proposals_dir`, then `Error::external`, then `Error::io` (both in [[Error]]).
3. `run` in [[codewiki]] and `reject_proposal` in [[Api Module]] call `reject`. `reject` calls `get`, then `Error::invalid` in [[Error]], then `write_json`.
4. `generate_with` in [[Generate Module]] calls `create`. `create` calls `proposals_dir`, then `Error::io` in [[Error]], then `list`, then `remove`, then `slug`, then `body_hash` in [[Manifest Module]], and 2 more. `remove` calls `proposals_dir`, then `Error::io` in [[Error]].
5. `get_proposal` in [[Api Module]] calls `get`. `get` calls `check_id`, then `proposals_dir`, then `Error::not_found` in [[Error]], then `read_json`. `check_id` calls `Error::invalid` in [[Error]].

2 more entry points: `proposed_text`, `patch_text`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub enum ProposalStatus` — public enum; used by `Proposal`.
- `pub struct Proposal` — public struct; used by `ProposalDetail`, `approve`, `approve_proposal`, `create` and 7 more.
- `fn proposals_dir(vault: &Path) -> PathBuf` — private function, returns `PathBuf`; called by `create`, `get`, `list`, `patch_text` and 3 more.
- `pub fn unified_diff(old: &str, new: &str, path: &str) -> String` — unified diff between two note texts, for display.
- `fn read_json(path: &Path) -> Result<Proposal>` — private function, returns `Result<Proposal>`; called by `get`, `list`.
- `fn write_json(vault: &Path, proposal: &Proposal) -> Result<()>` — private function, returns `Result<()>`; called by `approve`, `create`, `reject`.
- `create` — queue a proposal. `current` is the note text on disk, empty for a new note. Any earlier pending proposal for the same note is replaced.
- `pub fn list(vault: &Path) -> Result<Vec<Proposal>>` — all proposals, sorted by id.
- `pub fn get(vault: &Path, id: &str) -> Result<Proposal>` — public function, returns `Result<Proposal>`; called by `approve`, `get_proposal`, `reject`.
- `pub fn proposed_text(vault: &Path, id: &str) -> Result<String>` — proposed full text of the note.
- `pub fn patch_text(vault: &Path, id: &str) -> Result<String>` — public function, returns `Result<String>`; called by `get_proposal`.
- `fn slug(note_id: &str) -> String` — File-name-safe form of a note id such as `type:cw_core::Error`.
- `fn check_id(id: &str) -> Result<()>` — ids come from API callers; never let one name a path outside the folder.
- `fn remove(vault: &Path, id: &str) -> Result<()>` — private function, returns `Result<()>`; called by `create`.
- `pub fn approve(vault: &Path, id: &str) -> Result<Proposal>` — apply a pending proposal: write the note and record the outcome. Fails with the proposal marked `Superseded` when the note changed meanwhile.
- `pub fn reject(vault: &Path, id: &str) -> Result<Proposal>` — reject a pending proposal. The caller records `input_hash` on the note entry so the same change is not proposed again.

## Depends on

- [[Error]] — uses `external`, `invalid`, `io`, `not_found`
- [[Error Module]] — uses `Result`
- [[Manifest Module]] — uses `body_hash`
- [[VaultMeta]] — uses `dir`

## Used by

- [[Api Module]] — from `ProposalDetail`, `approve_proposal`, `get_proposal`, `list_proposals`
- [[cw-wiki]] — from `human_edited_notes_get_a_proposal_not_an_overwrite`, `reports_a_missing_wiki_then_its_state`
- [[codewiki]] — from `run`
- [[Briefing Module]] — from `session_start`
- [[Generate Module]] — from `generate_with`
- [[Query Module]] — from `status`

## See also

- [[_index]]
- [[cw-vault]]
