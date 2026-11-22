---
tags: [core, codewiki-app]
---

# codewiki-app

> CodeWiki desktop app for Windows.

## What it does

A native viewer for the vault and the live index. All data comes from the backend over HTTP; this binary links none of the engine crates. The one thing it does outside the backend is update its own installation (`cw-update`).

## Source

- `crates/codewiki-app/Cargo.toml` — package manifest
- `crates/codewiki-app/src/main.rs` — crate root

## How it works

Builds a binary.

1. The program starts at `main`. `main` calls `read` in [[Prefs Module]], then `level_named` in [[Logs Module]], then `init` in [[cw-telemetry]], then `acquire` in [[Instance Module]], then `launch` in [[Startup Module]], then `hold` in [[Instance Module]], and 12 more.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Modules:

- [[Autostart Module]] — starting the app when the user signs in to Windows.
- [[Backdrop Module]] — the drifting backdrop: soft color under the glass.
- [[Build Module]] — embeds the app's icon in the Windows program.
- [[Contrast Module]] — WCAG contrast arithmetic for translucent surfaces.
- [[Diagram View Module]] — where an enlarged diagram sits in the diagram viewer.
- [[Diagrams Module]] — drawn Mermaid diagrams, kept so that a note opens without waiting.
- [[Doc Module]] — what a generated page says, apart from how it is drawn.
- [[Doc View Module]] — draws a `Doc` with the `mui` components.
- [[Editor Module]] — text logic for the note editor: wikilink completion and line endings.
- [[Fonts Module]] — fonts bundled in the binary.
- [[Glass Module]] — glass: how see-through each surface is.
- [[Graph Module (codewiki-app)]] — geometry of the dependency graph view.
- [[Icons Module]] — icons, and the asset source that serves them.
- [[Illustration Module]] — native renderer for the pictures of a note.
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
- [[Zoom Module]] — zoom: the whole app larger or smaller, as a browser zooms a page.

Defined at the crate root:

- `const STATUS_PAUSE: std::time::Duration` — how often the app asks the backend what it is doing, for the tray icon.
- `const OLD_BACKEND_PAUSE: std::time::Duration` — the same, of a backend too old to say: it will not learn to.
- `const TRAY_HINT: &str` — the setting that remembers that the user was told, once, that closing the window leaves the app running in the tray.
- `const APP_IDENTITY: &str` — what Windows knows the app as when it shows a notification.
- `const MIN_WINDOW: (f32, f32)` — the smallest window: the sidebar, the details pane, and a reading pane about as wide as the two together.
- `const SCREEN_MARGIN: f32` — kept free around the first window on a screen it would otherwise fill. A window as large as the free area has its unseen frame, and with it a few pixels of the title bar, outside the screen.
- `const WIKI_MAP: &str` — the vault's map note, which the app calls "Wiki map".
- `enum Landing` — where the app goes once a repository's notes have loaded.
- `enum Event` — what reaches the app from outside GPUI: from another thread, or from code that runs with no way to the app's state. Each is sent through a channel and carried out by one task of the app's view (`AppView::listen`), so that none of it ever runs while GPUI is in the middle of something.
- `struct Tab` — A tab that is not on screen. The active tab's page and history live in the view itself; its slot is refreshed when the user leaves it.
- `struct DiagramViewer` — A note being edited. A diagram of the page, opened over the window to be enlarged.
- `enum Page` — what the centre pane shows.
- `const SIDEBAR_WIDTH: f32` — width of the sidebar and of the details pane, in pixels.
- `fn parse_repos(value: &Value) -> (Vec<Repo>, Vec<Repo>)` — the registered repositories, split into those whose folders are there and those whose folders are gone. A backend from before the list said so counts every folder as there.
- `fn registered_repos(backend: &Backend) -> Result<(Vec<Repo>, Vec<Repo>), String>` — the registered repositories whose folders are there, and those whose folders are not found. Nothing is forgotten here: a folder can be away for a while. Blocks; run it off the UI thread.
- `fn theme_menu_item(entry: &themes::Entry, in_use: bool, cx: &App) -> MenuItem` — A theme as a row of a menu: its icon in a color of its own, its name, a sample of its four colors, and a mark on the one in use. The toolbar's theme menu and the theme select of the settings list themes with it, so that a theme looks the same in both.
- `fn zone_label(zone: &str) -> String` — `01-Core` -> `Core`.
- `fn min_window_size() -> Size<Pixels>` — the smallest the window may be made. The window's own measures are the screen's pixels, whatever the zoom.
- `fn stops_backend_on_quit(remote: bool, installing: bool) -> bool` — whether ending the app also stops the backend. `remote` says that the backend is the one `CODEWIKI_URL` names; `installing` that an update is being installed or waits for a restart.
- `fn main()` — private function.

And 19 more items.

## Depends on

- [[Live Module]] — uses `HomeInput`, `Kept`, `OPEN`, `Project`
- [[Graph Module (codewiki-app)]] — uses `Camera`, `DETAIL_LIMIT`, `GraphData`, `cell_size`
- [[Menu Module]] — uses `AnchorBounds`, `Menu`, `MenuItem`, `MenuSection`
- [[Settings View Module]] — uses `JOB_PAUSE`, `LOG_LEVEL_PREF`, `OpenMenu`, `State`
- [[Pages Module]] — uses `contracts_page`, `coupling_page`, `coverage_page`, `dead_code_page`
- [[Doc View Module]] — uses `Controls`, `DocPage`, `OnAction`, `PageSlot`
- [[Diagram View Module]] — uses `SHARP_SCALE`, `STEP`, `WHEEL`, `actual`
- [[Backend]] — uses `Backend`, `answers`, `connect`, `delete`

## Used by

- [[Settings View Module]] — from `new`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
