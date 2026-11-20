---
tags: [core, cw-wiki, module]
---

# Generate Module

> The generation pipeline: scan, plan, freeze names, render, lint, write.

## What it does

One function serves first-time generation, incremental updates and adoption of an existing vault. Whether a note is written, proposed or left alone is decided per note by `cw_vault::lifecycle`.

## Source

- `crates/cw-wiki/src/generate.rs` — module `cw_wiki::generate`

## How it works

1. `run_update` in [[Service Module]] calls `generate_with`. `generate_with` calls `Error::io` in [[Error]], then `VaultMeta::load` in [[VaultMeta]], then `adopt_existing`, then `Repo::at` in [[Repo]], then `plan` in [[Plan Module]], then `describe` in [[Manifests Module]], and 18 more. `adopt_existing` calls `list_notes`, then `NoteKind::from_file_name` in [[NoteKind]], then `Error::io` in [[Error]], then `Note::parse` in [[Note]], then `adopted_kind`, then `cited_sources`, and 1 more.
2. `read_note_as` in [[Query Module]] calls `illustration_path`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const TEMPLATE_VERSION: u32` — bump when rendering changes in a way that should refresh every note.
- `pub enum Rewrite` — what a `ProseWriter` made of one note.
- `pub struct Stopped` — why a run stopped writing prose.
- [[ProseWriter]] — replaces the structural prose of a note, typically with an LLM.
- `pub struct WriterUsage` — what a writer spent during one run.
- `pub struct WriterContext<'a>` — facts handed to a `ProseWriter`.
- `pub struct Report` — public struct; used by `adopt_existing`, `generate`, `generate_with`, `print_report` and 4 more.
- `fn list_notes(vault: &Path) -> Result<Vec<String>>` — Vault-relative paths of every Markdown note on disk.
- `fn adopted_kind(path: &str, note: &Note) -> NoteKind` — kind of a note that CodeWiki did not write, from its name and tags.
- `fn cited_sources(note: &Note, file_hashes: &HashMap<&str, &str>) -> Vec<SourceRef>` — source files named in a note's `## Source` section that exist in the repo.
- `adopt_existing` — register notes found on disk that the manifest does not know.
- `generate` — generate or update the vault of the repository at `root`, reading nothing but the repository: a package from outside it is described only by what an earlier run kept in the vault.
- `generate_with` — `generate`, with `packages` to say what a package from outside the repository is.
- `pub fn illustration_path(note_path: &str) -> String` — vault path of the illustration that belongs to the note at `note_path`.
- `fn write_readme(vault: &Path, repo_name: &str) -> Result<()>` — the vault's own README explains how to open it; written once, never updated.

And 3 more private items with no comment.

## Depends on

- [[ProseWriter]] — uses `ProseWriter`, `illustrate`, `name`, `rewrite`
- [[VaultMeta]] — uses `VaultMeta`, `load`, `registry`, `save`
- [[Manifest Module]] — uses `NoteEntry`, `SourceRef`, `body_hash`
- [[Manifests Module]] — uses `NoPackageInfo`, `PackageInfo`, `describe`
- [[Registry]] — uses `entries`, `new`, `reserve`
- [[RenderCtx]] — uses `illustration`, `new`, `render`
- [[cw-illustrate]] — uses `EXTENSION`, `check`
- [[Lifecycle Module]] — uses `action`, `state`

## Used by

- [[cw-wiki]] — from `an_integration_note_says_what_is_taken_and_at_which_version`, `illustrate`, `rewrite`, `run`
- [[LlmWriter]] — from `LlmWriter`, `illustrate`, `record`, `rewrite`
- [[Service Module]] — from `run_update`, `update`, `update_in`
- [[ProseWriter]] — from `illustrate`, `rewrite`, `usage`
- [[Api Module]] — from `run_update`, `update_repo`
- [[codewiki]] — from `print_report`
- [[Writer Module]] — from `read_sources`
- [[Query Module]] — from `read_note_as`

## See also

- [[_index]]
- [[cw-wiki]]
