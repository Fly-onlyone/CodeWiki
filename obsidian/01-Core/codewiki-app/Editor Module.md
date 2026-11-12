---
tags: [core, codewiki-app, module]
---

# Editor Module

> Text logic for the note editor: wikilink completion and line endings.

## Source

- `crates/codewiki-app/src/editor.rs` — module `codewiki_app::editor`

## How it works

Main items:

- `restore_line_endings` — give `edited` the line endings `original` used.
- `completions` — note names to offer for `query`, at most `limit`: names starting with the query first, then names containing it, each group in the given order.
- `wikilink_query` — the partly typed wikilink target before the cursor, if the cursor is inside an open `((`.

## Used by

- [[codewiki-app]] — from `edit_view`, `save_edit`

## See also

- [[_index]]
- [[codewiki-app]]
