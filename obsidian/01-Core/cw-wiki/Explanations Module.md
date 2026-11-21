---
tags: [core, cw-wiki, module]
---

# Explanations Module

> Explanations: what a coding agent writes into a note, and how.

## What it does

Rules write every note from the code. They state facts and cannot say what a thing is for. A coding agent can: it reads the code. This module is its way in, and the only one:

- `queue` lists the notes that wait for an explanation.
- `brief` gives the agent one note: its text, which sections are the agent's, the files to read, and the limits.
- `submit` takes the agent's sections and pictures. It checks them and weaves them into the note. The agent never sends a whole note, so it cannot change a fact.

A note its owner edited is never written: what the agent sends for it is filed as a proposal for the owner to review.

## Source

- `crates/cw-wiki/src/explanations.rs` — module `cw_wiki::explanations`

## How it works

1. `session_start` in [[Briefing Module]] calls `waiting_counts`. `waiting_counts` calls `VaultMeta::load` in [[VaultMeta]], then `count`, then `all_waiting`. `all_waiting` calls `waits`.
2. `list_explanations` in [[Api Module]] calls `queue`. `queue` calls `VaultMeta::load` in [[VaultMeta]], then `all_waiting`, then `count`, then `tidy`, then `title_of` in [[Query Module]].
3. `read_explanation` in [[Api Module]] calls `brief`. `brief` calls `VaultMeta::load` in [[VaultMeta]], then `find_entry` in [[Query Module]], then `not_explainable`, then `note_text`, then `Error::not_found` in [[Error]], then `split` in [[Note Module]], and 8 more. `not_explainable` calls `Error::invalid` in [[Error]]. `note_text` calls `Error::io` in [[Error]].
4. `write_explanation` in [[Api Module]] calls `submit`. `submit` calls `VaultMeta::load` in [[VaultMeta]], then `find_entry` in [[Query Module]], then `not_explainable`, then `note_text`, then `body_hash` in [[Manifest Module]], then `changed_since_sync`, and 17 more. `changed_since_sync` calls `file_hash` in [[Generate Module]].
5. `approve_proposal` in [[Api Module]] calls `approve`. `approve` calls `pictures_text`, then `approve` (both in [[Proposal Module]]), then `VaultMeta::load` in [[VaultMeta]], then `proposed_text` in [[Proposal Module]], then `agent_name`, then `record`. `record` calls `illustration_path` in [[Generate Module]], then `Error::io` in [[Error]], then `body_hash` in [[Manifest Module]].

1 more entry point: `reject`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const DEFAULT_LIMIT: usize` — notes a queue lists when the caller names no number.
- `pub const MAX_LIMIT: usize` — notes a queue lists at most. An agent writes a note at a time, and each costs its user's allowance.
- `pub const MAX_SECTION_WORDS: usize` — words an agent's text for one section holds at most.
- [[Counts]] — how many notes wait for an agent.
- `pub enum Why` — why a note waits.
- `pub struct Waiting` — one note of the queue.
- `pub struct Scope` — which waiting notes a queue lists.
- `pub fn waiting_counts(root: &Path) -> Result<Counts>` — how many notes of the repository at `root` wait for an agent.
- `pub fn queue(root: &Path, scope: &Scope) -> Result<Queue>` — the notes that wait for an agent, as far as `scope` asks.
- `pub struct Yours` — one of the agent's sections of a note, in a `Brief`.
- `pub struct BriefSource` — A file a note is about, in a `Brief`.
- `pub struct Limits` — what a submission must stay within.
- `pub struct Brief` — all an agent needs to explain one note.
- `pub fn brief(root: &Path, path_or_name: &str) -> Result<Brief>` — what an agent needs to explain the note at vault path or name `path_or_name`.
- `pub struct Written` — the text an agent wrote for one section.
- `pub struct Submission` — what an agent sends for one note.
- `pub enum Outcome` — what became of a submission. Each is an answer, not an error: an agent acts on every one of them.
- `pub fn submit(root: &Path, path_or_name: &str, submission: &Submission) -> Result<Outcome>` — take what an agent wrote for the note at vault path or name `path_or_name`: check it, weave it into the note, save its pictures and record what it was written from. See `Outcome` for what can come of it. The search index is the caller's to refresh after `Saved`.
- `pub fn approve(root: &Path, id: &str) -> Result<proposal::Proposal>` — approve the pending proposal `id`: write the note, and for an agent's proposal its pictures and the record of who explained it. After a sync's proposal the note counts as reviewed for that input, so the same change is not proposed again. After an agent's proposal the note stays its owner's: it was edited by hand and nothing overwrites it, so the next change of its facts is a proposal again.
- `pub fn reject(root: &Path, id: &str) -> Result<proposal::Proposal>` — reject the pending proposal `id`. A sync's input is recorded, so the same change is not proposed again. An agent's text is dropped, and an agent may offer another.

And 15 more items.

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

- [[cw-wiki]] — from `a_brief_says_which_sections_are_the_agents`, `a_note_its_owner_edited_gets_a_proposal_and_is_not_written`, `a_note_that_is_behind_comes_first_and_is_cleared_by_writing_again`, `a_submission_against_an_older_note_or_older_facts_is_turned_back`
- [[Api Module]] — from `approve_proposal`, `list_explanations`, `read_explanation`, `reject_proposal`
- [[codewiki]] — from `write_notes_skill_follows_the_tools_and_the_limits`
- [[Briefing Module]] — from `session_start`

## See also

- [[_index]]
- [[cw-wiki]]
