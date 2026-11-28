---
tags: [core, codewiki-app, module]
---

# Ready Module

> The lines the app writes to its log when a page has been drawn with its data.

## What it does

A program that drives the app from outside cannot see what the window shows, and UI Automation cannot read it. Such a program waits for these lines instead of a fixed time: `Wait-AppPage` in `scripts/vm/test-release.ps1`.

A line names the kind of the page and, for a note, the note's name. It never holds a search, a palette query, a file's path or the title of a generated page, which can be any of them.

## Source

- `crates/codewiki-app/src/ready.rs` — module `codewiki_app::ready`

## How it works

1. `AppView::announce_page` in [[codewiki-app]] calls `kind`. `kind` calls `stem`.
2. `AppView::announce_page` in [[codewiki-app]] calls `drawn`.
3. `AppView::announce_page` in [[codewiki-app]] calls `filled`.
4. `AppView::new` and `AppView::open_request` in [[codewiki-app]] call `label`.

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

## See also

- [[_index]]
- [[codewiki-app]]
