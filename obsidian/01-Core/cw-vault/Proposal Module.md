---
tags: [core, cw-vault, module]
---

# Proposal Module

> The review queue: proposed changes to human-edited notes.

## What it does

The review queue for notes a person edited. CodeWiki never overwrites such a note. When new text is ready, a proposal stores it beside the note, with a diff to read. Nothing in the vault changes until a person approves.

Three kinds of proposal exist. A sync proposes when the code under a note changed. A coding agent proposes when it explained a note whose owner had edited it. A retire proposal asks to delete notes the owner wrote about code that is gone. All are stored as files and shown in the app.

## Source

- `crates/cw-vault/src/proposal.rs` — module `cw_vault::proposal`

## How it works

Each proposal lives in a `proposals` folder under the metadata folder, as files with one id: a `.json` record (the `Proposal` type), the proposed text as `.md`, a unified `.patch`, and for an agent with pictures an `.openui` file.

`create` queues a sync's proposal. `create_agent` queues an agent's, named by a hash of the proposed text and pictures, so the same offer twice is one proposal. Both go through `create_as`, which first deletes an older pending proposal of the same note and the same kind, then writes the files. The record stores `base_hash`, the hash of the note as it was. Ids differ by kind: a sync's is the note slug plus a hash, an agent's has `agent` in the middle.

`create_retire` takes a list of `Retired` notes and makes one proposal for all of them, with id `retire-` and a hash of their paths and base hashes. The same notes found again leave the proposal alone and return `None`. Different notes replace it. An empty list removes a pending retire proposal. `close_retire` records the decision; deleting the notes is the caller's work, since only the caller knows what git holds.

`approve` re-reads the note. If its hash is not `base_hash` any more, the proposal becomes `Superseded` and an error asks for a new proposal. Otherwise it writes the proposed text and marks the proposal `Approved`. `reject` marks it `Rejected`. `supersede` marks a pending proposal overtaken when what it says is no longer true. `pending_count` reads only the status of each record.

## Items

- `pub enum ProposalStatus` — public enum; used by `Proposal`, `Status`, `close_retire`.
- `pub enum ProposalKind` — what made a proposal.
- `pub struct Retired` — one note a retire proposal would delete.
- `pub struct Offer` — what an agent's proposal carries beside the note's text.
- `pub struct Proposal` — public struct; used by `ProposalDetail`, `approve`, `approve_keeping`, `approve_proposal` and 15 more.
- `pub const RETIRE_NOTE: &str` — the id every retire proposal's `note_id` holds: it stands for the vault.
- `pub fn unified_diff(old: &str, new: &str, path: &str) -> String` — unified diff between two note texts, for display.
- `create` — queue a sync's proposal. `current` is the note text on disk, empty for a new note. An earlier pending proposal of a sync for the same note is replaced; one of an agent stays.
- `create_agent` — queue an agent's proposal: the note with the agent's sections woven in, and what comes with it. An earlier pending proposal of an agent for the same note is replaced; one of a sync stays, since it says something else: that the facts changed.
- `pub fn create_retire(vault: &Path, mut notes: Vec<Retired>) -> Result<Option<Proposal>>` — queue the proposal to delete `notes`, which their owner wrote about code that is gone: one proposal for all of them, which a person decides as a whole, leaving out the notes they want to keep. With no notes nothing is queued. A pending retire proposal for other notes is replaced; one for these same notes stays as it is, and `None` says so.
- `pub fn close_retire(vault: &Path, id: &str, status: ProposalStatus) -> Result<Proposal>` — record the decision on the pending retire proposal `id`. What the decision does to the notes is the caller's work: it needs to know what git holds, which this crate does not.
- `pub fn list(vault: &Path) -> Result<Vec<Proposal>>` — all proposals, sorted by id.
- `pub fn pending_count(vault: &Path) -> Result<usize>` — how many proposals wait for a decision. Reads only the `status` of each proposal file, so a patch or a text is never loaded.
- `pub fn get(vault: &Path, id: &str) -> Result<Proposal>` — public function, returns `Result<Proposal>`; called by `approve`, `approve_keeping`, `close_retire`, `get_proposal` and 3 more.
- `pub fn proposed_text(vault: &Path, id: &str) -> Result<String>` — proposed full text of the note.
- `pub fn patch_text(vault: &Path, id: &str) -> Result<String>` — public function, returns `Result<String>`; called by `get_proposal`.
- `pub fn pictures_text(vault: &Path, id: &str) -> Result<String>` — the pictures file an agent's proposal would give the note; empty when it carries none.
- `pub fn approve(vault: &Path, id: &str) -> Result<Proposal>` — apply a pending proposal: write the note and record the outcome. Fails with the proposal marked `Superseded` when the note changed meanwhile.
- `pub fn supersede(vault: &Path, id: &str) -> Result<()>` — mark the pending proposal `id` as overtaken: what it would write is no longer true of the repository, so it can no longer be approved. A proposal that is not pending is left as it is.
- `pub fn reject(vault: &Path, id: &str) -> Result<Proposal>` — reject a pending proposal. The caller records `input_hash` on the note entry so the same change is not proposed again.

And 8 more items.

## Depends on

- [[Error]] — uses `external`, `invalid`, `io`, `not_found`
- [[Manifest Module]] — uses `Explained`, `body_hash`
- [[Error Module]] — uses `Result`
- [[Hash Module]] — uses `sha256_hex`
- [[VaultMeta]] — uses `dir`

## Used by

- [[cw-wiki]] — from `a_fault_the_owner_wrote_is_not_held_against_the_agent`, `a_note_its_owner_edited_gets_a_proposal_and_is_not_written`, `a_proposal_that_waits_for_a_note_of_something_gone_cannot_be_approved`, `an_explained_note_changed_by_hand_gets_a_proposal_with_the_explanation`
- [[Api Module]] — from `ProposalDetail`, `approve_proposal`, `get_proposal`, `list_proposals`
- [[Explanations Module]] — from `approve`, `approve_keeping`, `reject`, `submit`
- [[Gone Module]] — from `keep_all`, `pending_retire`, `retire`
- [[codewiki]] — from `run`
- [[cw-server]] — from `repo_with_proposals`
- [[Briefing Module]] — from `session_start`
- [[Generate Module]] — from `generate_with`

## Gotchas

- A sync proposal and an agent proposal for the same note live side by side. A new one replaces only a pending proposal of its own kind.
- Only pending proposals can be approved or rejected. Any other state is an error.
- A retire proposal can be neither approved nor rejected with `approve` or `reject`; both refuse it. Use `close_retire`.
- A decided retire proposal is never replaced or removed by a later sync.
- `check_id` allows only letters, digits, dash and underscore. Ids come from API callers, so one must never name a path outside the folder.
- A new note has an empty `base_hash`. Approval then expects that the file does not exist yet.
- `approve` writes the text but does not update the note's manifest entry. The caller, for example `approve` in [[Explanations Module]], does that.

## See also

- [[_index]]
- [[cw-vault]]
