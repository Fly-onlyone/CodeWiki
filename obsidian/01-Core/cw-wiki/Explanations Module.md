---
tags: [core, cw-wiki, module]
---

# Explanations Module

> Explanations: what a coding agent writes into a note, and how.

## What it does

Rules write every note from the code, so they state facts and cannot say what a thing is for. A coding agent can, because it reads the code. This module is the agent's only way in. It lists the notes that wait, hands the agent one note with its limits, takes back the sections the agent wrote, and decides what happens to the review proposals that follow.

The agent never sends a whole note, only its own sections, so it cannot change a fact. A note its owner edited is never written; the agent's text is filed as a proposal for the owner instead.

## Source

- `crates/cw-wiki/src/explanations.rs` — module `cw_wiki::explanations`

## How it works

A note waits when its kind can be explained, it has not been kept after its subject went, nobody edited it, and either no agent explained it (`Unexplained`) or an explained note has reasons it may be behind (`Behind`). `all_waiting` lists them, behind first, then by path. `queue` filters by folder and source files, drops notes that another agent holds, and cuts to the limit (clamped to 1 through `MAX_LIMIT`).

`brief` returns the note text, the agent's sections, the files to read, the limits, and a `base`: the body hash of the note as it is now.

`submit` goes through gates in order:

- The note file must exist, else `SyncFirst`.
- `base` must equal the hash of the current text, else `Changed`.
- No source file may have changed since the last sync, else `SyncFirst`.
- Each section must be one the agent owns, sent once, non-empty, at most the word limit, with no `#` or `##` heading, and no code block where code is a fact.
- The woven note must pass the strict linter. A fault the note already had is ignored, so the agent is not blamed for an owner's mistake.
- Pictures must name an owned section that exists, at most one per section, and pass `cw_illustrate::review`.

If all pass, the text is woven in, the pictures file is written, and `record` stores who explained it, from which source hashes, and at which commit, and clears the behind reasons. Pictures of sections not sent are kept. A note edited by its owner, or adopted, is not written: `create_agent` files a proposal, and the outcome is `Proposed`.

`approve_keeping` sends a retire proposal to the `gone` module, which deletes the notes not kept. `reject` of a retire proposal keeps all its notes as history. `decide_all` runs a batch and reports each result.

## Items

- `pub const DEFAULT_LIMIT: usize` — notes a queue lists when the caller names no number.
- `pub const MAX_LIMIT: usize` — notes a queue lists at most. An agent writes a note at a time, and each costs its user's allowance.
- `pub const MAX_SECTION_WORDS: usize` — words an agent's text for one section holds at most.
- [[Counts]] — how many notes wait for an agent.
- `pub enum Why` — why a note waits.
- `pub struct Waiting` — one note of the queue.
- `pub struct Queue` — public struct; used by `every`, `list_explanations`, `queue`.
- `pub struct Scope` — which waiting notes a queue lists.
- `pub fn waiting_counts(root: &Path) -> Result<Counts>` — how many notes of the repository at `root` wait for an agent.
- `pub fn queue(root: &Path, scope: &Scope) -> Result<Queue>` — the notes that wait for an agent, as far as `scope` asks. Notes another agent holds are left out before the limit applies.
- `pub fn every(root: &Path, claimed: &[String]) -> Result<Queue>` — every waiting note, with no limit and no filter, in queue order. Notes in `claimed` are marked.
- `pub fn brief(root: &Path, path_or_name: &str) -> Result<Brief>` — what an agent needs to explain the note at vault path or name `path_or_name`.
- `pub struct Written` — the text an agent wrote for one section.
- `pub struct Submission` — what an agent sends for one note.
- `pub enum Outcome` — what became of a submission. Each is an answer, not an error: an agent acts on every one of them.
- `pub fn submit(root: &Path, path_or_name: &str, submission: &Submission) -> Result<Outcome>` — take what an agent wrote for the note at vault path or name `path_or_name`: check it, weave it into the note, save its pictures and record what it was written from. See `Outcome` for what can come of it. The search index is the caller's to refresh after `Saved`.
- `pub fn approve(root: &Path, id: &str) -> Result<proposal::Proposal>` — approve the pending proposal `id`: write the note, and for an agent's proposal its pictures and the record of who explained it. After a sync's proposal the note counts as reviewed for that input, so the same change is not proposed again. After an agent's proposal the note stays its owner's: it was edited by hand and nothing overwrites it, so the next change of its facts is a proposal again.
- `pub fn approve_keeping(root: &Path, id: &str, keep: &[String]) -> Result<proposal::Proposal>` — `approve`, for a retire proposal with some of its notes left out: `keep` holds the vault paths of the notes that stay, as history. The others are deleted where git holds them (`crate::gone`). Any other proposal is about one note and takes no `keep`.
- `pub fn reject(root: &Path, id: &str) -> Result<proposal::Proposal>` — reject the pending proposal `id`. A sync's input is recorded, so the same change is not proposed again. An agent's text is dropped, and an agent may offer another.
- `pub fn decide_all(root: &Path, decision: Decision, ids: &[String]) -> Vec<Decided>` — apply `decision` to each of the proposals `ids`, in order. A proposal that is missing, superseded or already decided is reported and the rest still run. The caller holds the repository's write lock for the whole batch, and indexes the notes once afterwards when an approval went through.

And 23 more items.

## Depends on

- [[Proposal Module]] — uses `Offer`, `Proposal`, `approve`, `create_agent`
- [[cw-illustrate]] — uses `MAX_PICTURES`, `library`, `limits`, `parse_pictures`
- [[NoteKind]] — uses `NoteKind`, `agent_sections`, `explainable`, `fact_fences`
- [[Error]] — uses `Error`, `invalid`, `io`, `not_found`
- [[VaultMeta]] — uses `VaultMeta`, `load`, `registry`, `save`
- [[Explanation Module]] — uses `extract`, `weave`
- [[Manifest Module]] — uses `Explained`, `body_hash`
- [[Generate Module]] — uses `file_hash`, `illustration_path`

## Used by

- [[cw-wiki]] — from `a_brief_says_which_sections_are_the_agents`, `a_note_changed_after_the_proposal_is_not_deleted_by_it`, `a_note_its_owner_edited_gets_a_proposal_and_is_not_written`, `a_note_that_is_behind_comes_first_and_is_cleared_by_writing_again`
- [[Api Module]] — from `approve_proposal`, `decide_proposals`, `list_explanations`, `read_explanation`
- [[Briefing Module]] — from `explain_sentence`, `launch_order`, `session_start`
- [[codewiki]] — from `batch_and_claim_texts_follow_the_backend`, `write_notes_skill_follows_the_tools_and_the_limits`

## Gotchas

- Every refusal is an `Outcome`, not an error. The agent is expected to act on each one.
- A section sent again without a picture loses its old picture, since that picture explained the old text.
- Files the agent names as read are hashed and added as sources, up to `MAX_READ` (40). Changing one later marks the note behind.
- An approved agent proposal leaves the note in its owner's hands: `approve` restores the owner's body hash. The next change of its facts becomes a proposal again, never an overwrite.
- A rejected sync proposal records its input hash, so the same change is not proposed again. A rejected agent proposal records nothing; an agent may offer another.
- Only `approve_keeping` may take a `keep` list, and only for a retire proposal.
- The caller must refresh the search index after `Saved`; this module does not.
- A section may not hold a `#` or `##` heading, because it would end the section or start one of the rules'.

## See also

- [[_index]]
- [[cw-wiki]]
