---
tags: [core, cw-wiki, module]
---

# Decisions Module

> Decision records and the "why" query.

## What it does

Decisions are read from two places: notes tagged `decision` in the vault, and ADR files the repository already keeps (`docs/adr/`, `decisions/`). A decision goes stale when the code it cites changes after it was written.

## Source

- `crates/cw-wiki/src/decisions.rs` — module `cw_wiki::decisions`

## How it works

1. `run` in [[codewiki]] and `list_decisions` in [[Api Module]], and 1 more, call `list`. `list` calls `load` in [[Gitintel Module]], then `is_adr_path`, then `build`, then `VaultMeta::load` in [[VaultMeta]], then `Note::parse` in [[Note]]. `build` calls `Note::parse` in [[Note]], then `cited_paths`, then `status_of`.
2. `run` in [[codewiki]] and `create_decision` in [[Api Module]] call `create`. `create` calls `sanitize_name` in [[Registry Module]], then `Error::invalid`, then `Error::io` (both in [[Error]]).
3. `run` in [[codewiki]] and `why` in [[Api Module]] call `why`. `why` calls `Error::not_found` in [[Error]], then `list`, then `VaultMeta::load` in [[VaultMeta]], then `Note::parse` in [[Note]], then `recent_commits`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const DECISIONS_ZONE: &str` — vault folder new decisions are written to.
- `pub struct Decision` — public struct; used by `Why`, `build`, `list`, `list_decisions`.
- `pub struct Rationale` — public struct; used by `Why`, `why`.
- `pub struct CommitInfo` — public struct; used by `Why`, `recent_commits`.
- `pub struct Why` — public struct; used by `why`.
- `fn is_adr_path(path: &str) -> bool` — is this repo path an ADR file by the usual conventions?
- `fn status_of(text: &str, note: &Note) -> String` — status from a `Status: accepted` line or the first line under `## Status`.
- `fn cited_paths(text: &str, index_files: &[String]) -> (Vec<String>, Vec<String>)` — backticked spans that name files in the repository, split into existing and missing.
- `fn build(path: &str, text: &str, index_files: &[String], intel: &GitIntel) -> Decision` — private function, returns `Decision`; called by `list`.
- `pub fn list(root: &Path, index: &Index) -> Result<Vec<Decision>>` — every decision record in the repository and its vault, by path.
- `pub fn create(root: &Path, title: &str) -> Result<String>` — create a new decision note from the standard template and return its vault-relative path. The note is the author's to fill in.
- `fn recent_commits(root: &Path, file: &str, limit: usize) -> Vec<CommitInfo>` — private function, returns `Vec<CommitInfo>`; called by `why`.
- `pub fn why(root: &Path, index: &Index, target: &str) -> Result<Why>` — why is this code the way it is? `target` is a file path or a symbol name. Collects the decisions that cite the file, the rationale recorded in notes about it, and the latest commits that touched it.

## Depends on

- [[Error]] — uses `invalid`, `io`, `not_found`
- [[Index]] — uses `Index`, `files`, `find_symbols`
- [[Gitintel Module]] — uses `GitIntel`, `load`
- [[VaultMeta]] — uses `load`, `notes_for_source`
- [[Note]] — uses `Note`, `parse`
- [[cw-core]] — uses `VAULT_DIR`
- [[Error Module]] — uses `Result`
- [[Registry Module]] — uses `sanitize_name`

## Used by

- [[cw-wiki]] — from `a_decision_goes_stale_when_its_code_changes_later`, `lists_decisions_from_adr_files_and_vault_notes`, `why_gathers_decisions_rationale_and_commits`
- [[Api Module]] — from `create_decision`, `list_decisions`, `why`
- [[codewiki]] — from `run`
- [[Ci Module]] — from `check`

## See also

- [[_index]]
- [[cw-wiki]]
