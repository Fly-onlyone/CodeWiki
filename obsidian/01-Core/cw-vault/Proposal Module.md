---
tags: [core, cw-vault, module]
---

# Proposal Module

> The review queue: proposed changes to human-edited notes.

## Source

- `crates/cw-vault/src/proposal.rs` — module `cw_vault::proposal`

## How it works

Main items:

- `ProposalStatus` — enum
- `Proposal` — struct
- `list` — all proposals, sorted by id.
- `get` — function
- `reject` — reject a pending proposal.
- `proposed_text` — proposed full text of the note.
- `patch_text` — function
- `approve` — apply a pending proposal: write the note and record the outcome.

## Depends on

- [[Error]] — uses `external`, `invalid`, `io`, `not_found`
- [[cw-llm]] — uses `header`
- [[Error Module]] — uses `Result`
- [[Manifest Module]] — uses `body_hash`
- [[VaultMeta]] — uses `dir`

## Used by

- [[Api Module]] — from `ProposalDetail`, `approve_proposal`, `get_proposal`, `list_proposals`
- [[cw-wiki]] — from `human_edited_notes_get_a_proposal_not_an_overwrite`, `reports_a_missing_wiki_then_its_state`
- [[codewiki]] — from `run`
- [[Briefing Module]] — from `session_start`
- [[Generate Module]] — from `generate`
- [[Query Module]] — from `status`

## See also

- [[_index]]
- [[cw-vault]]
