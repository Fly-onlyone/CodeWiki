---
tags: [core, codewiki-app, module]
---

# Ready Module

> The lines the app writes to its log when a page has been drawn with its data.

## What it does

A program that drives the app from outside cannot see what the window shows, and UI Automation cannot read it. This module writes the lines such a program waits for instead of sleeping a fixed time. The VM test script waits for them with `Wait-AppPage`.

A line says that a page was drawn with its data, names the kind of page and, for a note, the note's name. It never holds a search, a palette query, a file's path or the title of a generated page.

## Source

- `crates/codewiki-app/src/ready.rs` — module `codewiki_app::ready`

## How it works

The app calls `kind` with the page it shows and `Facts`, what it holds of the data. `kind` returns `None` while the page still lacks its data: Home waits for a backend answer and, with a repository open, for the list of notes; Waiting waits for the waiting list; Settings waits for the settings answer. A plain `Message` page is never announced, because it stands in for a page still to come. Graph, Review, Generated and Notice are announced at once.

When `kind` gives a value, `drawn` makes the line, such as `page drawn: note _HOME`, with `(asked ...)` added when the page answers a launch argument. `label` makes that text from the argument. `filled` makes the line for data that arrives after the draw: `page filled: diagrams` or `page filled: models`.

`Holds` says what keeps the shown page from being the one a launch asked for: running work, a view, editor or palette not yet opened, or a change made in this frame that shows only in the next. While `Holds::any` is true the page is not announced.

All lines go to the log under the target `page`. `stem` turns a vault path into the note's name.

## Items

- `pub const TARGET: &str` — the log target of the lines. `cw_telemetry` leaves it out of the error reports, so a note's name stays on the machine.
- `pub enum Kind` — the kind of a page, as its line names it.
- `pub enum Fill` — what arrives after a page was drawn and changes it.
- `pub struct Facts` — what the app holds of the data its pages are built from.
- `pub struct Holds` — what still keeps the page that is shown from being the one a launch asked for.
- `pub fn kind(page: &Page, facts: Facts) -> Option<Kind>` — the kind of `page`, once the data it shows has arrived. A plain message is never announced: it stands in for a page that is still to come.
- `pub fn drawn(kind: &Kind, asked: Option<&str>) -> String` — the line for a page that was drawn. `asked` is the label of the view a launch named, when this page is the answer to it.
- `pub fn filled(fill: Fill) -> String` — the line for something that arrived after its page was drawn.
- `pub fn label(argument: &str) -> String` — the label of the view a launch named, as `drawn` repeats it: the argument as it was given, without a palette query or a file's path, and `default` for a launch that named none.
- `fn stem(path: &str) -> &str` — the name of the note at the vault path `path`: its file name without `.md`.

## Depends on

- [[codewiki-app]] — uses `Page`
- [[Category]] — uses `slug`

## Used by

- [[codewiki-app]] — from `AppView`, `announce_page`, `new`, `open_request`

## Gotchas

- The lines are an interface. A test rig matches their exact text, so changing a word breaks it.
- `label` keeps only the word `palette:` or `file:` and drops what follows, because a query or a path can hold private text. Every other argument is repeated as given.
- The note's name is in the line, so the target `page` is left out of Sentry reports. Keep that, or a note's name would leave the machine.
- A page whose data never arrives is never announced. A rig waits until its own timeout.
- `stem` accepts both slash styles, so a Windows path gives the same name.

## See also

- [[_index]]
- [[codewiki-app]]
