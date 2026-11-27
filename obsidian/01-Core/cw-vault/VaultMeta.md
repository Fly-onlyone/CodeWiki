---
tags: [core, cw-vault]
---

# VaultMeta

> Everything under `.codewiki/`, loaded.

## What it does

The loaded state of a vault's metadata folder: the manifest plus a map of every [[NoteEntry]] by note id. Almost every part of CodeWiki that touches notes loads it first, edits it in memory, and saves it back.

## Source

- `crates/cw-vault/src/manifest.rs` — definition of `VaultMeta`

## How it works

`load` reads `manifest.json` and `notes.jsonl` from the metadata folder. A missing file gives the default, so a new vault starts empty. A broken file or line is an error. `save` creates the folder, writes the manifest, writes the entries sorted by id, and writes `registry.json`, derived from the entries by `registry` so the two cannot disagree. `by_path` and `notes_for_source` search the entries. `notes_for_source` returns the notes that cite a given source file, which is how a code change finds the notes it affects. `dir` gives the metadata folder path and is also used for the proposals folder.

## Items

Declared as `pub struct VaultMeta`.

Members:

- `pub fn dir(vault: &Path) -> PathBuf` — public method, returns `PathBuf`; called by `VaultMeta::load`, `VaultMeta::save`, `proposals_dir`.
- `pub fn load(vault: &Path) -> Result<Self>` — load metadata; a vault with none yields the default (empty) state.
- `pub fn save(&self, vault: &Path) -> Result<()>` — write metadata. `registry.json` is derived from the note entries so the two can never disagree.
- `pub fn registry(&self) -> Result<Registry>` — build the name registry from the note entries.
- `pub fn live_notes(&self) -> usize` — how many notes the wiki has: those that describe something. A note kept after what it describes went is in the vault and not of the wiki, whatever the reason it stays for.
- `pub fn by_path(&self, path: &str) -> Option<&NoteEntry>` — public method, returns `Option<&NoteEntry>`.
- `pub fn notes_for_source(&self, source_path: &str) -> Vec<&NoteEntry>` — ids of notes that cite `source_path`.

## Depends on

- [[Manifest Module]] — uses `Json`, `Manifest`, `default`
- [[Registry]] — uses `Registry`, `entries`, `new`
- [[Error]] — uses `external`, `io`
- [[cw-core]] — uses `MANIFEST_DIR`
- [[Error Module]] — uses `Result`
- [[NoteEntry]] — uses `NoteEntry`
- [[RegistryEntry]] — uses `RegistryEntry`

## Used by

- [[cw-wiki]] — from `a_move_with_two_possible_ends_carries_nothing`, `a_new_way_of_writing_the_facts_keeps_the_explanation`, `a_note_that_waits_to_be_retired_is_no_note_of_the_wiki`, `a_run_counts_the_notes_no_agent_has_explained`
- [[Explanations Module]] — from `all_waiting`, `approve_keeping`, `brief`, `fits_all`
- [[Query Module]] — from `find_entry`, `list_notes`, `notes_citing`, `read_note_as`
- [[Generate Module]] — from `adopt_existing`, `carry_moves`, `generate_with`
- [[Gone Module]] — from `after_removal`, `keep_all`, `retire`
- [[Decisions Module]] — from `list`, `why`
- [[Service Module]] — from `index_notes`, `is_stale`
- [[codewiki]] — from `run`

## Gotchas

- The whole set of entries is held in memory, and `save` rewrites all three files each time. Two writers at once can overwrite each other.
- `by_path` and `notes_for_source` scan every entry. They are linear in the number of notes.
- `registry` can fail if two entries clash in name, and then `save` fails too.
- Entries are written sorted, which keeps git diffs small and stable.

## See also

- [[_index]]
- [[Manifest Module]]
