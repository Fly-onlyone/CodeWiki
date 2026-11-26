---
tags: [core, codewiki-app, module]
---

# Editor Module

> Text logic for the note editor: wikilink completion and line endings.

## What it does

The app lets the user edit a note's Markdown. This module holds the pure text logic behind that editor, with no window code, so tests can run it directly. It does two jobs: it helps the author type a wikilink (`[[Note Name]]`) by offering matching note names, and it keeps a note's original line endings when the edited text is saved.

## Source

- `crates/codewiki-app/src/editor.rs` — module `codewiki_app::editor`

## How it works

While the user types, `edit_view` calls `wikilink_query` with the text and the cursor byte offset. If the cursor sits inside an open `[[` on the current line, the function returns what has been typed after it, such as `Par`. Then `completions` filters the list of note names: names that start with the query come first, then names that contain it, each group in the order given, cut to the limit.

On save, `save_edit` calls `restore_line_endings`. The editor works with `\n`. If the original text held `\r\n`, the edited text is turned back to `\r\n`, so a save changes only the lines the author touched.

## Items

- `pub fn wikilink_query(text: &str, cursor: usize) -> Option<&str>` — the partly typed wikilink target before the cursor, if the cursor is inside an open `[[`. `cursor` is a byte offset into `text`. `see [[Par|` gives `Par`; a closed link, a line break or an alias part (`[[Name|`) before the cursor gives `None`.
- `pub fn completions<'a>(names: &'a [String], query: &str, limit: usize) -> Vec<&'a str>` — note names to offer for `query`, at most `limit`: names starting with the query first, then names containing it, each group in the given order. Matching ignores case. Map notes (`_index`, `_HOME`) are offered only when the query starts with an underscore.
- `pub fn restore_line_endings(original: &str, edited: &str) -> String` — give `edited` the line endings `original` used. The editor works with `\n`; a note saved with `\r\n` must keep them so an edit changes only the lines the author touched.

## Used by

- [[codewiki-app]] — from `edit_view`, `save_edit`

## Gotchas

- `cursor` is a byte offset. A cursor inside a multi-byte character is not a valid position and gives `None`.
- `wikilink_query` looks only at the current line. A closed link, a `|` alias part or a line break before the cursor ends the query.
- Map notes (names that start with `_`, such as `_index`) are offered only when the query itself starts with an underscore.
- Matching ignores case, but the names are returned as given.
- `restore_line_endings` decides by whether the original contains any `\r\n`. A file with mixed endings comes back with `\r\n` everywhere.

## See also

- [[_index]]
- [[codewiki-app]]
