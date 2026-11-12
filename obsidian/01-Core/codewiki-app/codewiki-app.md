---
tags: [core, codewiki-app]
---

# codewiki-app

> CodeWiki desktop app for Windows.

## Source

- `crates/codewiki-app/Cargo.toml` — package manifest
- `crates/codewiki-app/src/main.rs` — crate root

## How it works

Builds a binary.

Modules:

- [[Editor Module]] — text logic for the note editor: wikilink completion and line endings.
- [[Graph Module (codewiki-app)]] — geometry of the dependency graph view.
- [[Illustration Module]] — native renderer for illustrations.
- [[Markdown Module]] — text preparation for the note viewer.
- [[Pages Module]] — Markdown for the report pages: decisions, dead code, health, drift, settings.
- [[Palette Module]] — the command palette's entries and matching.
- [[Themes Module]] — bundled color themes and theme switching.

Defined at the crate root:

- `AppView` — struct
- `Page` — what the centre pane shows.
- `Tab` — A tab that is not on screen.
- `NoteInfo` — struct
- `Repo` — struct
- `Editing` — A note being edited.
- `zone_label` — `01-Core` -> `Core`.
- `parse_notes` — function

## Depends on

- [[Markdown Module]] — uses `DIAGRAM_SCHEME`, `FILE_SCHEME`, `NOTE_SCHEME`, `SYMBOL_SCHEME`
- [[Pages Module]] — uses `contracts_page`, `coupling_page`, `coverage_page`, `dead_code_page`
- [[Graph Module (codewiki-app)]] — uses `DETAIL_LIMIT`, `cell_size`, `density_cells`, `label_candidates`
- [[Palette Module]] — uses `Command`, `Entry`, `actions`, `filter`
- [[Backend]] — uses `Backend`, `connect`, `put`, `repo_id`
- [[Camera]] — uses `Camera`, `fit`, `pan`, `zoom_at`
- [[Editor Module]] — uses `completions`, `restore_line_endings`, `wikilink_query`
- [[Themes Module]] — uses `init`, `list`, `select`

## Used by

- [[cw-server]] — from `call`, `hook`
- [[Api Module]] — from `require_token`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
