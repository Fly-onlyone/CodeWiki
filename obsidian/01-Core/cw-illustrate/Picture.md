---
tags: [core, cw-illustrate]
---

# Picture

> One picture of a note: the `## ` section it explains, and its source.

## What it does

`Picture` is one saved picture of a wiki note. It pairs the name of the note section the picture stands in with the picture's source, a short OpenUI Lang program. A note keeps its pictures as a list of these. The app draws each one inside its section.

## Source

- `crates/cw-illustrate/src/lib.rs` — definition of `Picture`

## How it works

Pictures of a note live in one file with the extension `openui`. `serialize_pictures` writes each picture under a line `# section: <name>`, with a blank line between. `parse_pictures` reads the file back: a marker line starts a new picture and the lines after it are its source. OpenUI Lang treats a line starting with `#` as a comment, so every picture in the file is a valid program by itself. Text before the first marker, a marker with no name, and a marker with no source are dropped. The [[Explanations Module]] builds `Picture` values when an agent submits a note, and the query and API code send them to the app.

## Items

Declared as `pub struct Picture`.

## Used by

- [[Explanations Module]] — from `Brief`, `Submission`, `pictures_of`, `submit`
- [[cw-illustrate]] — from `parse_pictures`, `serialize_pictures`
- [[cw-wiki]] — from `picture`
- [[Api Module]] — from `ProposalDetail`
- [[Query Module]] — from `NoteContent`

## Gotchas

- The section is the heading without the two hash signs, for example "How it works".
- The file format depends on the marker `# section: `. `review` in [[cw-illustrate]] refuses any picture line that starts with it, because that line would split the file.
- Writing no pictures gives an empty string, and then no file is kept.
- Source is trimmed on write and read, so trailing blank lines do not survive.

## See also

- [[_index]]
- [[cw-illustrate]]
