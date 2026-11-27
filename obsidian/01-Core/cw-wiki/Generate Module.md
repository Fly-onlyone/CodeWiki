---
tags: [core, cw-wiki, module]
---

# Generate Module

> The generation pipeline: scan, plan, freeze names, render, lint, write.

## What it does

This is the engine that turns scanned code into a vault of notes. It runs the same steps for a first generation, an update after code changes, and a vault that already holds hand-written notes. For each note it decides whether to write it, merge new facts into it, propose a change, or leave it alone. It also tracks which notes an agent explained that may now be behind the code. It returns a `Report` that says what happened.

## Source

- `crates/cw-wiki/src/generate.rs` — module `cw_wiki::generate`

## How it works

`generate_with` does these steps in order.

- Load the manifest. `adopt_existing` registers Markdown notes on disk the manifest does not know. Their kind comes from file name or tags, and their sources from the backticked paths in their Source section.
- Ask the plan module which notes should exist. Files covered by adopted notes get no new notes.
- Freeze names. Existing notes keep their paths. A planned note that collides with an adopted note takes over its entry. New notes get a free name from the registry.
- Render every draft and lint it in strict mode. Any lint error aborts the whole run (the first 20 are shown). Warnings are only counted.
- Notes CodeWiki wrote whose subject is gone are marked as leaving, unless a person edited them. This is done before links are woven, so links to them can be removed.
- For each planned note, hash the draft with the template version. A new note is written. A known note goes through `lifecycle::state` and `lifecycle::action`: keep, merge, propose or regenerate. A merge takes the new facts, keeps the agent's sections (`weave_kept`), and keeps the pictures whose section still exists (`keep_placed_pictures`).
- Then the behind check. For a note an agent explained, each file it was written from is hashed again. A changed or missing file puts a line in `NoteEntry::behind` and the note in `Report::behind`. A line that says a section moved stays until an agent writes the note again.
- Picture files of notes no agent explained are deleted. Notes that are leaving are removed with their picture file.

`file_hash` re-hashes one repository file the way a scan does, so other modules can tell whether a source changed since the last sync.

## Items

- `pub const TEMPLATE_VERSION: u32` — bump when rendering changes in a way that should refresh every note.
- [[Report]] — struct
- `fn some_of(paths: &[String]) -> String` — the first few of `paths`, and how many more there are.
- `fn notes(count: usize) -> String` — `1 note` or `3 notes`.
- `pub struct KeptGone` — A note a sync left in the vault although its subject is gone.
- `pub struct Moved` — A note that a sync moved because its subject moved.
- `fn list_notes(vault: &Path) -> Result<Vec<String>>` — Vault-relative paths of every Markdown note on disk.
- `fn adopted_kind(path: &str, note: &Note) -> NoteKind` — kind of a note that CodeWiki did not write, from its name and tags.
- `fn cited_sources(note: &Note, file_hashes: &HashMap<&str, &str>) -> Vec<SourceRef>` — source files named in a note's `## Source` section that exist in the repo.
- `adopt_existing` — register notes found on disk that the manifest does not know.
- `fn movable(id: &str) -> Option<(&str, &str)>` — what a note describes, for the kinds of subject that can move: the kind and the name alone, without the place. `type:engine::pricing::Price` gives `("type", "Price")`.
- `carry_moves` — give each note whose subject moved to the planned note of that subject in its new place, so that what was written in it is kept. A subject that moves gets a new id, because the id names the place. The old note is then a note of something gone and the new one starts empty. Here the old entry takes the new id and the new path instead. Only a clear match counts: git saw the file move to the one planned note's file, or exactly one new note and one old note share kind and name. The files are not touched here. Returns the renames to make, old and new vault path, once the run can no longer fail before it writes.
- `generate` — generate or update the vault of the repository at `root`, reading nothing but the repository: a package from outside it is described only by what an earlier run kept in the vault.
- `generate_with` — `generate`, with `packages` to say what a package from outside the repository is.
- `const CARRIED: &str` — how a line of `NoteEntry::behind` starts that says the note followed its subject to another place in the code.
- `pub(crate) fn file_hash(root: &Path, path: &str) -> Option<String>` — the hash of the repository file `path` as it is now, made as a scan makes it; nothing for a file that is gone or leaves the repository.
- `struct Links<'a>` — which links of a note lead to a note once this run is over.
- `weave_kept` — the fresh `draft` of a note with the sections an agent wrote, as they stand in `current`, the note on disk. A link of the agent's to a note that is gone becomes plain words. With `owners_too`, for a note its owner edited: a section that rules do not write at all, such as `Gotchas`, is kept as well. It can only be the owner's, and the draft has nothing to put in its place.
- `fn keep_placed_pictures(vault: &Path, entry: &mut NoteEntry, text: &str) -> Result<()>` — keep the pictures of a note whose section the note `text` still has. A picture stands inside a section and has no place without it.
- `pub fn illustration_path(note_path: &str) -> String` — vault path of the file that holds the pictures of the note at `note_path`.

And 5 more items.

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

- [[Explanations Module]] — from `brief`, `changed_since_sync`, `pictures_of`, `record`
- [[cw-wiki]] — from `an_explanation_follows_its_subject_to_a_new_place`, `an_integration_note_says_what_is_taken_and_at_which_version`
- [[Report]] — from `Report`, `summarize`
- [[Gone Module]] — from `retire`
- [[Query Module]] — from `read_note_as`
- [[Service Module]] — from `run_update`
- [[Git]] — from `holds`

## Gotchas

- A style error in any draft stops the whole run before any note is written.
- Raise `TEMPLATE_VERSION` when rendering changes and every note must refresh. It is part of every input hash.
- First contact with an adopted note only records its input hash and keeps it. Only later source changes raise a proposal.
- Two notes with the same name cannot both be managed. The second is skipped and listed in the report.
- Folders starting with a dot, `_Templates` and `_Attachments` are not adopted.
- When the action is keep, the note's recorded sources are still refreshed to the current hashes. Without that, a brief would ask for a sync that changes nothing.
- Rules draw no pictures. A picture file of a note that no agent explained is deleted on every run.
- A note whose owner edited it is not counted as unexplained, since an agent's text for it would only be a proposal.
- `file_hash` returns nothing for a missing file or one outside the repository.

## See also

- [[_index]]
- [[cw-wiki]]
