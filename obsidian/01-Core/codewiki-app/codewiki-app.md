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

- [[Autostart Module]] — starting the app when the user signs in to Windows.
- [[Backdrop Module]] — the drifting backdrop: soft color under the glass.
- [[Build Module]] — embeds the app's icon in the Windows program.
- [[Contrast Module]] — WCAG contrast arithmetic for translucent surfaces.
- [[Diagrams Module]] — drawn Mermaid diagrams, kept so that a note opens without waiting.
- [[Doc Module]] — what a generated page says, apart from how it is drawn.
- [[Doc View Module]] — draws a `Doc` with the `mui` components.
- [[Editor Module]] — text logic for the note editor: wikilink completion and line endings.
- [[Fonts Module]] — fonts bundled in the binary.
- [[Glass Module]] — glass: how see-through each surface is.
- [[Graph Module (codewiki-app)]] — geometry of the dependency graph view.
- [[Icons Module]] — icons, and the asset source that serves them.
- [[Illustration Module]] — native renderer for illustrations.
- [[Instance Module]] — one running app for each installed program.
- [[Live Module]] — the live pages: one symbol, one file, search results, hotspots, the theme picker, a proposal, the result of a sync, and Home, the page the app starts on.
- [[Markdown Module]] — text preparation for the note viewer.
- [[Motion Module]] — motion: whether things move at all.
- [[Mui Module]] — components drawn to Material UI's measurements.
- [[Pages Module]] — the report pages: decisions, dead code, health, drift and the rest.
- [[Palette Module]] — the command palette's entries and matching.
- [[Panes Module]] — the two side panes: whether each is shown.
- [[Prefs Module]] — small per-user settings, one value per file under `%APPDATA%\CodeWiki`.
- [[Reading Module]] — typography of the reading pane.
- [[Settings Module]] — what the Settings page is made of, apart from how it is drawn: its categories, the fields a user types into, and the news shown beside one.
- [[Settings Layout Module]] — the measures of the Settings page that are a matter of taste.
- [[Settings Pages Module]] — the pages of the Settings: one for each `Category`, built from what the app knows and from what the backend says about its own settings (`GET /v1/settings`).
- [[Settings View Module]] — the Settings page as the app's view draws and runs it.
- [[Standard Module]] — standard levels for a theme the app did not ship.
- [[Startup Module]] — which repository the app opens when it starts.
- [[Status Module]] — what the app says about itself in the tray: what the backend is doing and whether an agent is using it.
- [[Themes Module]] — bundled color themes and theme switching.
- [[Tray Module]] — the app's icon in the notification area, with its tooltip and its menu.
- [[Update Module]] — updating the app: when to look for a new release, and what the settings page says about it.
- [[Win32 Module]] — the Win32 calls GPUI has no way to make.

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
- [[Menu Module]] — uses `AnchorBounds`, `Menu`, `MenuItem`, `MenuSection`
- [[Live Module]] — uses `HomeInput`, `Project`, `file_page`, `home_page`
- [[Pages Module]] — uses `contracts_page`, `coupling_page`, `coverage_page`, `dead_code_page`
- [[Doc View Module]] — uses `Controls`, `DocPage`, `OnAction`, `PageSlot`
- [[Settings View Module]] — uses `LOG_LEVEL_PREF`, `MODELS_PAUSE`, `MODEL_HINT`, `OpenMenu`
- [[Glass Module]] — uses `Levels`, `OPAQUE`, `chrome`, `for_window`
- [[Update Module]] — uses `State`, `auto_check`, `available`, `can_check`

## Used by

- [[Settings View Module]] — from `new`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
