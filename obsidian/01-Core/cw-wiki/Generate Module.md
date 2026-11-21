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

1. `pictures_of` and `record` in [[Explanations Module]], and 1 more, call `illustration_path`.
2. `run_update` in [[Service Module]] calls `generate_with`. `generate_with` calls `Error::io` in [[Error]], then `VaultMeta::load` in [[VaultMeta]], then `adopt_existing`, then `Repo::at` in [[Repo]], then `plan` in [[Plan Module]], then `describe` in [[Manifests Module]], and 20 more. `adopt_existing` calls `list_notes`, then `NoteKind::from_file_name` in [[NoteKind]], then `Error::io` in [[Error]], then `Note::parse` in [[Note Module]], then `adopted_kind`, then `cited_sources`, and 1 more.
3. `brief` and `changed_since_sync` in [[Explanations Module]], and 1 more, call `file_hash`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const TEMPLATE_VERSION: u32` — bump when rendering changes in a way that should refresh every note.
- `pub struct Report` — public struct; used by `adopt_existing`, `generate`, `generate_with`, `print_report` and 4 more.
- `fn list_notes(vault: &Path) -> Result<Vec<String>>` — Vault-relative paths of every Markdown note on disk.
- `fn adopted_kind(path: &str, note: &Note) -> NoteKind` — kind of a note that CodeWiki did not write, from its name and tags.
- `fn cited_sources(note: &Note, file_hashes: &HashMap<&str, &str>) -> Vec<SourceRef>` — source files named in a note's `## Source` section that exist in the repo.
- `adopt_existing` — register notes found on disk that the manifest does not know.
- `generate` — generate or update the vault of the repository at `root`, reading nothing but the repository: a package from outside it is described only by what an earlier run kept in the vault.
- `generate_with` — `generate`, with `packages` to say what a package from outside the repository is.
- `const MOVED: &str` — how a line of `NoteEntry::behind` starts that says a section of the agent's lost its place.
- `pub(crate) fn file_hash(root: &Path, path: &str) -> Option<String>` — the hash of the repository file `path` as it is now, made as a scan makes it; nothing for a file that is gone or leaves the repository.
- `struct Links<'a>` — which links of a note lead to a note once this run is over.
- `weave_kept` — the fresh `draft` of a note with the sections an agent wrote, as they stand in `current`, the note on disk. A link of the agent's to a note that is gone becomes plain words. With `owners_too`, for a note its owner edited: a section that rules do not write at all, such as `Gotchas`, is kept as well. It can only be the owner's, and the draft has nothing to put in its place.
- `fn keep_placed_pictures(vault: &Path, entry: &mut NoteEntry, text: &str) -> Result<()>` — keep the pictures of a note whose section the note `text` still has. A picture stands inside a section and has no place without it.
- `pub fn illustration_path(note_path: &str) -> String` — vault path of the file that holds the pictures of the note at `note_path`.
- `fn write_readme(vault: &Path, repo_name: &str) -> Result<()>` — the vault's own README explains how to open it; written once, never updated.

And 3 more private items with no comment.

## Depends on

- [[Explanation Module]] — uses `Woven`, `extract`, `unlink_dead`, `weave`
- [[VaultMeta]] — uses `VaultMeta`, `load`, `registry`, `save`
- [[NoteKind]] — uses `NoteKind`, `agent_sections`, `explainable`, `from_file_name`
- [[cw-illustrate]] — uses `EXTENSION`, `parse_pictures`, `serialize_pictures`
- [[Note Module]] — uses `Note`, `parse`, `split`
- [[Manifests Module]] — uses `NoPackageInfo`, `PackageInfo`, `describe`
- [[Plan Module]] — uses `Plan`, `Planned`, `plan`
- [[Registry]] — uses `Registry`, `new`, `reserve`

## Used by

- [[Explanations Module]] — from `brief`, `changed_since_sync`, `pictures_of`, `record`
- [[cw-wiki]] — from `an_integration_note_says_what_is_taken_and_at_which_version`, `run`, `run_settled`, `sync`
- [[Service Module]] — from `run_update`, `update`, `update_in`
- [[Api Module]] — from `run_update`, `update_repo`
- [[codewiki]] — from `print_report`
- [[Query Module]] — from `read_note_as`

## See also

- [[_index]]
- [[cw-wiki]]
