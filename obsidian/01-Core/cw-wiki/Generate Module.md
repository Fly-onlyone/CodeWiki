---
tags: [core, cw-wiki, module]
---

# Generate Module

> The generation pipeline: scan, plan, freeze names, render, lint, write.

## What it does

This is the engine that turns scanned code into a vault of notes. It runs the same steps for a first generation, an update after code changes, and a vault that already holds hand-written notes. For each note it decides whether to write it, merge new facts into it, propose a change, or leave it alone. It tracks which notes an agent explained that may now be behind the code. It also handles notes whose subject moved or is gone: it carries them to the new place, keeps them, or proposes to retire them. It returns a `Report` that says what happened.

## Source

- `crates/cw-wiki/src/generate.rs` — module `cw_wiki::generate`

## How it works

`generate_with` does these steps in order.

- Load the manifest. `adopt_existing` registers Markdown notes on disk the manifest does not know. Their kind comes from file name or tags, and their sources from the backticked paths in their Source section.
- Ask the plan module which notes should exist. Files covered by adopted notes get no new notes.
- Follow subjects. `carry_moves` gives a note whose type, module or pattern moved to the new planned note, when git saw the move or exactly one old and one new note share kind and name. `follow_units` moves notes into the folder of a renamed unit.
- Freeze names. Existing notes keep their paths. New notes get a free name from the registry.
- Judge hand-written notes. One that names only paths that are gone is queued, with the others, in one retire proposal; none is deleted by a run.
- Render every draft and lint it in strict mode. Any lint error aborts the whole run (the first 20 are shown).
- For each planned note, hash the draft with the template version. A new note is written. A known note goes through `lifecycle::state` and `lifecycle::action`: keep, merge, propose or regenerate. A merge (`weave_kept`) takes the new facts and keeps the agent's sections and pictures.
- Then the behind check. Each file an explanation was written from is hashed again. A changed or missing file puts a line in `NoteEntry::behind`. A hash made from raw bytes that still match is renewed to the hash a scan makes, so other line endings do not read as a change.
- Notes CodeWiki wrote whose subject is gone are removed, or kept and marked `gone`. Pending proposals for them are superseded, and `Report::summary` says what happened.

`file_hash` re-hashes one repository file the way a scan does. `hash_holds` also accepts the raw-byte hash.

## Items

- `pub const TEMPLATE_VERSION: u32` — bump when rendering changes in a way that should refresh every note.
- `pub struct Report` — public struct; used by `adopt_existing`, `carry_moves`, `follow_units`, `generate` and 6 more.
- `fn some_of(paths: &[String]) -> String` — the first few of `paths`, and how many more there are.
- `fn notes(count: usize) -> String` — `1 note` or `3 notes`.
- `pub struct KeptGone` — A note a sync left in the vault although its subject is gone.
- `pub struct Moved` — A note that a sync moved because its subject moved.
- `fn list_notes(vault: &Path) -> Result<Vec<String>>` — Vault-relative paths of every Markdown note on disk.
- `fn adopted_kind(path: &str, note: &Note) -> NoteKind` — kind of a note that CodeWiki did not write, from its name and tags.
- `fn cited_sources(note: &Note, file_hashes: &HashMap<&str, &str>) -> Vec<SourceRef>` — source files named in a note's `## Source` section that exist in the repo.
- `adopt_existing` — register notes found on disk that the manifest does not know.
- `fn unit_subfolder(path: &str) -> Option<&str>` — the folder a vault path is in, when it is below a unit of `01-Core`.
- `fn movable(id: &str) -> Option<(&str, &str)>` — what a note describes, for the kinds of subject that can move: the kind and the name alone, without the place. `type:engine::pricing::Price` gives `("type", "Price")`.
- `generate` — generate or update the vault of the repository at `root`, reading nothing but the repository: a package from outside it is described only by what an earlier run kept in the vault.
- `generate_with` — `generate`, with `packages` to say what a package from outside the repository is.
- `const CARRIED: &str` — how a line of `NoteEntry::behind` starts that says the note followed its subject to another place in the code.
- `fn file_bytes(root: &Path, path: &str) -> Option<Vec<u8>>` — the bytes of the repository file `path`; nothing for a file that is gone or leaves the repository.
- `pub(crate) fn file_hash(root: &Path, path: &str) -> Option<String>` — the hash of the repository file `path` as it is now, made as a scan makes it; nothing for a file that is gone or leaves the repository.
- `pub(crate) fn hash_holds(root: &Path, path: &str, hash: &str) -> bool` — whether the repository file `path` is as it was when the vault recorded `hash` for it. A hash recorded before a scan read CRLF as LF is of the file's raw bytes: it holds while those bytes stay the same, or every file with CRLF line endings would read as changed.
- `struct Links<'a>` — which links of a note lead to a note once this run is over.
- `pub fn illustration_path(note_path: &str) -> String` — vault path of the file that holds the pictures of the note at `note_path`.

And 9 more items.

## Depends on

- [[Gone Module]] — uses `Left`, `UNLINKED`, `after_removal`, `cited`
- [[Proposal Module]] — uses `Retired`, `create`, `create_retire`, `list`
- [[VaultMeta]] — uses `VaultMeta`, `live_notes`, `load`, `registry`
- [[Explanation Module]] — uses `Woven`, `extract`, `unlink_dead`, `weave`
- [[NoteKind]] — uses `NoteKind`, `agent_sections`, `explainable`, `from_file_name`
- [[Git]] — uses `Git`, `holds`, `moves`, `new`
- [[cw-illustrate]] — uses `EXTENSION`, `parse_pictures`, `serialize_pictures`
- [[Note Module]] — uses `Note`, `parse`, `split`

## Used by

- [[cw-wiki]] — from `an_explained_python_module_survives_the_change_of_its_module_path`, `an_explanation_follows_its_subject_to_a_new_place`, `an_integration_note_says_what_is_taken_and_at_which_version`, `kept_gone`
- [[Explanations Module]] — from `brief`, `changed_since_sync`, `pictures_of`, `record`
- [[Service Module]] — from `run_update`, `update`, `update_in`
- [[Api Module]] — from `run_update`, `update_repo`
- [[codewiki]] — from `print_report`
- [[Gone Module]] — from `retire`
- [[Query Module]] — from `read_note_as`
- [[Git]] — from `holds`

## Gotchas

- A style error in any draft stops the whole run before any note is written.
- Raise `TEMPLATE_VERSION` when rendering changes and every note must refresh. It is part of every input hash.
- First contact with an adopted note only records its input hash and keeps it. Only later source changes raise a proposal.
- A run never deletes a hand-written note. It proposes, and a person decides.
- When the target of a moved subject is not clear, the note is listed in `possible_moves` and nothing is carried over.
- Two notes with the same name cannot both be managed. The second is skipped and listed in the report.
- Folders starting with a dot, `_Templates` and `_Attachments` are not adopted.
- When the action is keep, the note's recorded sources are still refreshed to the current hashes. Without that, a brief would ask for a sync that changes nothing.
- Rules draw no pictures. A picture file of a note that no agent explained is deleted on every run.
- `file_hash` returns nothing for a missing file or one outside the repository.
- A hash taken from raw bytes holds only while those bytes stay the same. A sync renews it, so only a real change marks a note behind.

## See also

- [[_index]]
- [[cw-wiki]]
