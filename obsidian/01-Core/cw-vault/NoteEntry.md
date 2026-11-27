---
tags: [core, cw-vault]
---

# NoteEntry

> `NoteEntry` struct defined in `cw_vault::manifest`.

## What it does

One record in the vault's bookkeeping: everything CodeWiki remembers about a single note. Notes themselves carry only tags, so this record holds the note's id, path and kind, where it came from, which source files it cites, the hashes that tell whether it changed, and what an agent explained.

## Source

- `crates/cw-vault/src/manifest.rs` — definition of `NoteEntry`

## How it works

[[VaultMeta]] keeps entries in a map keyed by note id and writes them as one JSON line each. [[Lifecycle Module]] reads an entry to decide what may be done to the note.

- `body_hash` is the hash of the text as CodeWiki last wrote or adopted it. A file whose hash differs was edited by a person.
- `input_hash` is the hash of everything that fed the last generation. If new inputs hash the same, nothing is rewritten.
- `sources` lists the cited files as [[SourceRef]] items with the hash seen at that time. A different hash now means the note is stale.
- `explained` is set once an agent explained the note. After that, rules write only their own sections.
- `behind` holds one line per reason the explanation may be behind the code. It is rewritten at each sync and cleared when an agent explains the note again.
- `model` names the agent that explained the note, and `illustration` points to the file that keeps its pictures.
- `rejected_inputs` lists inputs for which a person rejected a proposal.

## Items

Declared as `pub struct NoteEntry`.

## Depends on

- [[Manifest Module]] — uses `Explained`, `Origin`
- [[Gone]] — uses `Gone`
- [[SourceRef]] — uses `SourceRef`
- [[NoteKind]] — uses `NoteKind`

## Used by

- [[Explanations Module]] — from `all_waiting`, `changed_since_sync`, `count`, `fits_all`
- [[Generate Module]] — from `adopt_existing`, `generate_with`, `keep_placed_pictures`, `weave_kept`
- [[VaultMeta]] — from `VaultMeta`, `by_path`, `load`, `notes_for_source`
- [[Query Module]] — from `find_entry`, `info`, `title_of`
- [[cw-wiki]] — from `entry`, `entry_of`
- [[Lifecycle Module]] — from `action`, `state`
- [[Gone Module]] — from `of_adopted`, `of_generated`
- [[Git]] — from `holds`

## Gotchas

- Optional fields are left out of the JSON when empty, so the file stays short and diffs stay small.
- `path` is relative to the vault, not to the repository.
- `origin` `Adopted` means the note existed before CodeWiki. It is treated as human-owned from the start.
- Changing the `body_hash` by hand makes CodeWiki think the note was edited.

## See also

- [[_index]]
- [[Manifest Module]]
