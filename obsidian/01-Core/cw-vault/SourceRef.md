---
tags: [core, cw-vault]
---

# SourceRef

> A source file a note describes, with the content hash seen at generation.

## What it does

Links a note to a source file it describes, together with the hash of that file at the time. Comparing the stored hash with the file's current hash is how CodeWiki learns that a note may be out of date.

## Source

- `crates/cw-vault/src/manifest.rs` — definition of `SourceRef`

## How it works

A [[NoteEntry]] holds a list of these, one for each file it cites. Each has the repository path, an optional list of symbol names, and the hash. The `Explained` record in [[Manifest Module]] holds the same kind of list for the files an agent read when it wrote an explanation. A changed hash on any of them marks the explanation as possibly behind the code.

## Items

Declared as `pub struct SourceRef`.

## Used by

- [[Generate Module]] — from `cited_sources`, `generate_with`
- [[Manifest Module]] — from `Explained`
- [[Explanations Module]] — from `submit`
- [[NoteEntry]] — from `NoteEntry`

## Gotchas

- The symbol list is left out of the saved JSON when empty.
- The hash is the one seen when the note was generated or explained. It is not refreshed until the note is written again.
- A file the agent read that is not a cited source of the note is still tracked through the `Explained` list.

## See also

- [[_index]]
- [[Manifest Module]]
