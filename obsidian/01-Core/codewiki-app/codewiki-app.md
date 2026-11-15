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

- [[Backdrop Module]] — the drifting backdrop: soft color under the glass.
- [[Build Module]] — embeds the app's icon in the Windows program.
- [[Contrast Module]] — WCAG contrast arithmetic for translucent surfaces.
- [[Doc Module]] — what a generated page says, apart from how it is drawn.
- [[Doc View Module]] — draws a `Doc` with the `mui` components.
- [[Editor Module]] — text logic for the note editor: wikilink completion and line endings.
- [[Fonts Module]] — fonts bundled in the binary.
- [[Glass Module]] — glass: how see-through each surface is.
- [[Graph Module (codewiki-app)]] — geometry of the dependency graph view.
- [[Icons Module]] — icons, and the asset source that serves them.
- [[Illustration Module]] — native renderer for illustrations.
- [[Live Module]] — the live pages: one symbol, one file, search results, hotspots, the theme picker, a proposal, the result of a sync, and Home, the page the app starts on.
- [[Logfile Module]] — the app's log: `%LOCALAPPDATA%\CodeWiki\logs\app.log`.
- [[Markdown Module]] — text preparation for the note viewer.
- [[Motion Module]] — motion: whether things move at all.
- [[Mui Module]] — components drawn to Material UI's measurements.
- [[Pages Module]] — the report pages: decisions, dead code, health, drift, settings and the rest.
- [[Palette Module]] — the command palette's entries and matching.
- [[Prefs Module]] — small per-user settings, one value per file under `%APPDATA%\CodeWiki`.
- [[Reading Module]] — typography of the reading pane.
- [[Standard Module]] — standard levels for a theme the app did not ship.
- [[Startup Module]] — which repository the app opens when it starts.
- [[Themes Module]] — bundled color themes and theme switching.
- [[Update Module]] — updating the app: when to look for a new release, and what the settings page says about it.

Defined at the crate root:

- `AppView` — struct
- `Page` — what the centre pane shows.
- `Repo` — struct
- `Tab` — A tab that is not on screen.
- `WIKI_MAP` — the vault's map note, which the app calls "Wiki map".
- `NoteInfo` — struct
- `registered_repos` — the registered repositories whose folders are there, and those whose folders are not found.
- `parse_repos` — the registered repositories, split into those whose folders are there and those whose folders are gone.

## Depends on

- [[Graph Module (codewiki-app)]] — uses `Camera`, `DETAIL_LIMIT`, `GraphData`, `cell_size`
- [[Pages Module]] — uses `contracts_page`, `coupling_page`, `coverage_page`, `dead_code_page`
- [[Live Module]] — uses `HomeInput`, `Project`, `file_page`, `home_page`
- [[Glass Module]] — uses `Levels`, `OPAQUE`, `chrome`, `for_window`
- [[Update Module]] — uses `State`, `auto_check`, `available`, `can_check`
- [[Backend]] — uses `Backend`, `answers`, `connect`, `delete`
- [[Doc View Module]] — uses `DocPage`, `OnAction`, `PageSlot`, `id`
- [[Menu]] — uses `Menu`, `new`, `on_close`, `on_highlight`

## Used by

- [[cw-server]] — from `call`, `hook`
- [[Api Module]] — from `require_token`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
