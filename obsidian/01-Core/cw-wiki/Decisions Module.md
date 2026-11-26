---
tags: [core, cw-wiki, module]
---

# Decisions Module

> Decision records and the "why" query.

## What it does

Teams record why they chose something in decision notes or ADR files. This module collects those records, tells you when one has gone out of date, creates new ones from a template, and answers the question "why is this code the way it is?" for a file or a symbol.

## Source

- `crates/cw-wiki/src/decisions.rs` — module `cw_wiki::decisions`

## How it works

`list` gathers decisions from three places: ADR files in the repository (folders named `adr`, `decisions` and similar, `is_adr_path`), vault notes of kind decision or tagged `decision` or `adr`, and any note in the vault decisions folder not yet in the manifest.

`build` turns each text into a `Decision`. It reads the status from a `Status:` line or the first line under a Status heading (default `accepted`). `cited_paths` takes backticked spans that look like file paths, outside code fences, and splits them into files that exist and files that do not.

A decision is stale when it cites a missing file, or when a cited file was last changed after the decision file itself, using git history. Superseded and deprecated decisions are never stale.

`create` makes `ADR-NNNN Title` in the decisions folder, numbering one above the highest existing number, from a fixed template with a Proposed status.

`why` resolves its target to a file (a path first, then a symbol name). It returns the decisions that cite that file, the Why, Gotchas and Don't sections of notes whose sources include it, and the last 8 commits from `git log`.

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
- [[Note Module]] — uses `Note`, `parse`
- [[Gitintel Module]] — uses `GitIntel`, `load`
- [[VaultMeta]] — uses `load`, `notes_for_source`
- [[cw-core]] — uses `VAULT_DIR`
- [[Error Module]] — uses `Result`
- [[Registry Module]] — uses `sanitize_name`

## Used by

- [[cw-wiki]] — from `a_decision_goes_stale_when_its_code_changes_later`, `lists_decisions_from_adr_files_and_vault_notes`, `why_gathers_decisions_rationale_and_commits`
- [[Api Module]] — from `create_decision`, `list_decisions`, `why`
- [[codewiki]] — from `run`
- [[Ci Module]] — from `check`

## Gotchas

- Staleness depends on git history. Without it, only missing citations make a decision stale.
- Only backticked paths with a slash and a file extension count as citations. Prose mentions do not.
- `recent_commits` returns an empty list when git is missing or fails; the error is not shown.
- A symbol name that matches several symbols resolves to one: the exact match, or the first fuzzy hit.
- The title is cleaned with `sanitize_name`; a title that cleans to nothing is refused.

## See also

- [[_index]]
- [[cw-wiki]]
