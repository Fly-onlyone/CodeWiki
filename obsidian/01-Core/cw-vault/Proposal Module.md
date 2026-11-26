---
tags: [core, cw-vault, module]
---

# Proposal Module

> The review queue: proposed changes to human-edited notes.

## What it does

The review queue for notes a person edited. CodeWiki never overwrites such a note. When new text is ready, a proposal stores it beside the note, with a diff to read. Nothing in the vault changes until a person approves. Two sources propose: a sync, when the code under a note changed, and a coding agent, when it explained a note whose owner had edited it. Both kinds are stored as files and shown in the app.

## Source

- `crates/cw-vault/src/proposal.rs` — module `cw_vault::proposal`

## How it works

Each proposal lives in a `proposals` folder under the metadata folder, as several files with one id: a `.json` record ([[Proposal Module]] holds the `Proposal` type), the proposed full text as `.md`, a unified `.patch`, and for an agent with pictures an `.openui` file.

`create` queues a sync's proposal. `create_agent` queues an agent's, naming it by a hash of the proposed text and pictures, so the same offer twice is one proposal. Both go through `create_as`, which first deletes an older pending proposal of the same note and the same kind, then writes the files. The record stores `base_hash`, the hash of the note as it was.

`approve` re-reads the note. If its hash is not the `base_hash` any more, the proposal becomes `Superseded` and an error says a new proposal is needed. Otherwise it writes the proposed text and marks the proposal `Approved`. `reject` marks it `Rejected`. The caller records the input hash on the note entry so the same change is not proposed again. `pending_count` reads only the status of each record.

## Items

- `pub enum ProposalStatus` — public enum; used by `Proposal`, `Status`.
- `pub enum ProposalKind` — what made a proposal.
- `pub struct Offer` — what an agent's proposal carries beside the note's text.
- `pub struct Proposal` — public struct; used by `ProposalDetail`, `approve`, `approve_proposal`, `create` and 9 more.
- `fn proposals_dir(vault: &Path) -> PathBuf` — private function, returns `PathBuf`; called by `create_as`, `get`, `list`, `patch_text` and 5 more.
- `pub fn unified_diff(old: &str, new: &str, path: &str) -> String` — unified diff between two note texts, for display.
- `fn read_json(path: &Path) -> Result<Proposal>` — private function, returns `Result<Proposal>`; called by `get`, `list`.
- `fn write_json(vault: &Path, proposal: &Proposal) -> Result<()>` — private function, returns `Result<()>`; called by `approve`, `create_as`, `reject`.
- `create` — queue a sync's proposal. `current` is the note text on disk, empty for a new note. An earlier pending proposal of a sync for the same note is replaced; one of an agent stays.
- `create_agent` — queue an agent's proposal: the note with the agent's sections woven in, and what comes with it. An earlier pending proposal of an agent for the same note is replaced; one of a sync stays, since it says something else: that the facts changed.
- `pub fn list(vault: &Path) -> Result<Vec<Proposal>>` — all proposals, sorted by id.
- `pub fn pending_count(vault: &Path) -> Result<usize>` — how many proposals wait for a decision. Reads only the `status` of each proposal file, so a patch or a text is never loaded.
- `pub fn get(vault: &Path, id: &str) -> Result<Proposal>` — public function, returns `Result<Proposal>`; called by `approve`, `get_proposal`, `reject`.
- `pub fn proposed_text(vault: &Path, id: &str) -> Result<String>` — proposed full text of the note.
- `pub fn patch_text(vault: &Path, id: &str) -> Result<String>` — public function, returns `Result<String>`; called by `get_proposal`.
- `pub fn pictures_text(vault: &Path, id: &str) -> Result<String>` — the pictures file an agent's proposal would give the note; empty when it carries none.
- `const PICTURES: &str` — extension of the file that holds a proposal's pictures.
- `fn check_id(id: &str) -> Result<()>` — ids come from API callers; never let one name a path outside the folder.
- `pub fn approve(vault: &Path, id: &str) -> Result<Proposal>` — apply a pending proposal: write the note and record the outcome. Fails with the proposal marked `Superseded` when the note changed meanwhile.
- `pub fn reject(vault: &Path, id: &str) -> Result<Proposal>` — reject a pending proposal. The caller records `input_hash` on the note entry so the same change is not proposed again.

And 3 more items.

## Depends on

- [[Error]] — uses `external`, `invalid`, `io`, `not_found`
- [[Manifest Module]] — uses `Explained`, `body_hash`
- [[Error Module]] — uses `Result`
- [[Hash Module]] — uses `sha256_hex`
- [[VaultMeta]] — uses `dir`

## Used by

- [[Api Module]] — from `ProposalDetail`, `approve_proposal`, `get_proposal`, `list_proposals`
- [[cw-wiki]] — from `a_fault_the_owner_wrote_is_not_held_against_the_agent`, `a_note_its_owner_edited_gets_a_proposal_and_is_not_written`, `an_explained_note_changed_by_hand_gets_a_proposal_with_the_explanation`, `human_edited_notes_get_a_proposal_not_an_overwrite`
- [[Explanations Module]] — from `approve`, `reject`, `submit`
- [[codewiki]] — from `run`
- [[cw-server]] — from `repo_with_proposals`
- [[Briefing Module]] — from `session_start`
- [[Generate Module]] — from `generate_with`
- [[Query Module]] — from `status`

## Gotchas

- A sync proposal and an agent proposal for the same note live side by side. Their ids differ, and a new one replaces only a pending proposal of its own kind.
- Only pending proposals can be approved or rejected. Any other state is an error.
- `check_id` allows only letters, digits, dash and underscore. Ids come from API callers, so one must never name a path outside the folder.
- A new note has an empty `base_hash`. Approval then expects that the file does not exist yet.
- `approve` writes the text but does not update the note's manifest entry. The caller, for example `approve` in [[Explanations Module]], does that.

## See also

- [[_index]]
- [[cw-vault]]
