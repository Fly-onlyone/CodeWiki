---
tags: [core, codewiki-app, module]
---

# Editor Module

> Text logic for the note editor: wikilink completion and line endings.

## Source

- `crates/codewiki-app/src/editor.rs` — module `codewiki_app::editor`

## How it works

1. `AppView::edit_view` in [[codewiki-app]] calls `wikilink_query`.
2. `AppView::edit_view` in [[codewiki-app]] calls `completions`.
3. `AppView::save_edit` in [[codewiki-app]] calls `restore_line_endings`.

## Items

- `pub fn wikilink_query(text: &str, cursor: usize) -> Option<&str>` — the partly typed wikilink target before the cursor, if the cursor is inside an open `[[`. `cursor` is a byte offset into `text`. `see [[Par|` gives `Par`; a closed link, a line break or an alias part (`[[Name|`) before the cursor gives `None`.
- `pub fn completions<'a>(names: &'a [String], query: &str, limit: usize) -> Vec<&'a str>` — note names to offer for `query`, at most `limit`: names starting with the query first, then names containing it, each group in the given order. Matching ignores case. Map notes (`_index`, `_HOME`) are offered only when the query starts with an underscore.
- `pub fn restore_line_endings(original: &str, edited: &str) -> String` — give `edited` the line endings `original` used. The editor works with `\n`; a note saved with `\r\n` must keep them so an edit changes only the lines the author touched.

## Used by

- [[codewiki-app]] — from `edit_view`, `save_edit`

## See also

- [[_index]]
- [[codewiki-app]]
