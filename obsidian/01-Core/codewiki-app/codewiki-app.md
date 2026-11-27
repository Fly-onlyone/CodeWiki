---
tags: [core, codewiki-app]
---

# codewiki-app

> CodeWiki desktop app for Windows.

## What it does

This is the Windows desktop app of CodeWiki. It shows the wiki notes and the live index of a repository in a native window, built on GPUI (a Rust UI toolkit) and gpui-kit. It lives in the tray, so closing the window hides it and the app keeps running.

All wiki data comes from the backend over HTTP through `cw-client`. The app links none of the engine crates (the indexer and the wiki writer). It uses `cw-core` for paths, names and the small files it reads itself, such as the list of folders kept from agents and the auto explain switch. It draws pictures and diagrams with `cw-illustrate`, `cw-mermaid` and `openui-lang`, sends error reports through `cw-telemetry`, and updates its own installation through `cw-update`. The crate root file holds `AppView`, the one big view that owns the pages, the tabs, the tray and the backend connection.

## Source

- `crates/codewiki-app/Cargo.toml` — package manifest
- `crates/codewiki-app/src/main.rs` — crate root

## How it works

`main` runs these steps before any window opens: it reads the log level and starts logging and, if the user allowed it, error reports (`cw_telemetry::init`). It makes the channel for events from outside GPUI. `instance::acquire` decides whether this launch is the main app; a second launch hands its request over and returns with no window. `win32::on_console_stop` sends `Event::Quit` on Ctrl+C. Inside the GPUI run come the module inits, the key bindings and the window with its `Shell`. The [[codewiki-app Startup Flow]] shows the order.

After that, `AppView` runs the app. `AppView::new` starts four jobs: `connect` reaches the backend, `start_updates` looks for new releases, `start_tray` puts the icon in the notification area, and `start_status_poll` asks the backend every 5 seconds what it does.

The status poll feeds the tray, the backdrop and the toolbar. It also reads the waiting notes again when the backend's note revision changes, and it drives `supervise`, which tries to reach a lost backend again. While an update installs, nothing is asked, because the update stops the backend.

`AppView::listen` is the one task that takes events from the channel. It hides or shows the window with the Win32 calls between two GPUI updates, then calls `handle_event`. Closing the window sends `Event::Hide` when a tray icon exists; without a tray, closing ends the app unless there are unsaved edits.

`notify_user` shows a toast in the window's corner while the window is on screen. While it is hidden or minimized it sends a Windows notification, and the click comes back as `Event::Notification`.

On quit, `on_app_quit` releases the single-instance lock, flushes error reports, and asks the backend to stop on a background task. That happens only when this app started it: `stops_backend_on_quit` is false for a backend named by `CODEWIKI_URL` and while an update installs.

## Items

Modules:

- [[Autostart Module]] — starting the app when the user signs in to Windows.
- [[Backdrop Module]] — the drifting backdrop: soft color under the glass.
- [[Build Module]] — embeds the app's icon in the Windows program.
- [[Connection Module]] — when the app tries to reach its backend again, and how.
- [[Constellation Module]] — the constellation behind the glass: a made-up graph of notes and the links between them, which the backdrop (`backdrop.rs`) draws over its soft color.
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
- [[Identity Module]] — the name and the icon Windows shows above the app's notifications.
- [[Illustration Module]] — native renderer for the pictures of a note.
- [[Instance Module]] — one running app for each installed program.
- [[Live Module]] — the live pages: one symbol, one file, search results, hotspots, the theme picker, a proposal, the result of a sync, the notes that wait for a coding agent, and Home, the page the app starts on.
- [[Markdown Module]] — text preparation for the note viewer.
- [[Motion Module]] — motion: whether things move at all.
- [[Mui Module]] — components drawn to Material UI's measurements.
- [[Outline Module]] — the outline in the details pane: the `##` headings of the note as its body draws them, which of their sections is being read, and the glide of the body to a heading the user clicks.
- [[Pages Module]] — the report pages: decisions, dead code, health, drift and the rest.
- [[Palette Module]] — the command palette's entries and matching.
- [[Panes Module]] — the two side panes: whether each is shown.
- [[Prefs Module]] — small per-user settings, one value per file under `%APPDATA%\CodeWiki` (`CodeWiki-Dev` for a development build, `cw_core::paths::APP_FOLDER`).
- [[Reading Module]] — typography of the reading pane.
- [[Review Module]] — the review queue as the app keeps it: the pending proposals, which of them are selected, which opens after a decision, and what the user is told.
- [[Review View Module]] — the review page as the app's view draws and runs it.
- [[Settings Module]] — what the Settings page is made of, apart from how it is drawn: its categories, the fields a user types into, and the news shown beside one.
- [[Settings Layout Module]] — the measures of the Settings page that are a matter of taste.
- [[Settings Pages Module]] — the pages of the Settings: one for each `Category`, built from what the app knows and from what the backend says about its own settings (`GET /v1/settings`).
- [[Settings View Module]] — the Settings page as the app's view draws and runs it.
- [[Standard Module]] — standard levels for a theme the app did not ship.
- [[Startup Module]] — which repository the app opens when it starts.
- [[Status Module]] — what the app says about itself in the tray: what the backend is doing and whether an agent is using it.
- [[Suggest Module]] — what the search field suggests while it is typed in: the hits of its text, looked up at each pause in typing, in a list under the field.
- [[Tab Ops Module]] — which tabs stay when some are closed: the logic of the tab menu, kept apart from the view so that it can be tested.
- [[Tab Prefs Module]] — the two tab settings: whether the app has tabs at all, and whether a note the user opens goes to a tab of its own.
- [[Themes Module]] — bundled color themes and theme switching.
- [[Toasts Module]] — short messages for the user: what to show and where it goes.
- [[Tray Module]] — the app's icon in the notification area, with its tooltip and its menu.
- [[Update Module]] — updating the app: when to look for a new release, and what the settings page says about it.
- [[Win32 Module]] — the Win32 calls GPUI has no way to make.
- [[Zoom Module]] — zoom: the whole app larger or smaller, as a browser zooms a page.

Defined at the crate root:

- `const SYNCING: &str` — the busy line while this app syncs a wiki.
- `const MIN_WINDOW: (f32, f32)` — the smallest window: the sidebar, the details pane, and a reading pane about as wide as the two together.
- `const SCREEN_MARGIN: f32` — kept free around the first window on a screen it would otherwise fill. A window as large as the free area has its unseen frame, and with it a few pixels of the title bar, outside the screen.
- `const WIKI_MAP: &str` — the vault's map note, which the app calls "Wiki map".
- `enum Landing` — where the app goes once a repository's notes have loaded.
- `struct Tab` — A tab that is not on screen. The active tab's page and history live in the view itself; its slot is refreshed when the user leaves it.
- `enum MenuTarget` — what a right-click opened a menu on.
- `struct Reader` — the note's text view, kept by the app so that the outline can read its headings and scroll it.
- `enum Page` — what the centre pane shows.
- `const SIDEBAR_WIDTH: f32` — width of the sidebar and of the details pane, in pixels.
- `fn parse_repos(value: &Value) -> (Vec<Repo>, Vec<Repo>)` — the registered repositories, split into those whose folders are there and those whose folders are gone. A backend from before the list said so counts every folder as there.
- `fn registered_repos(backend: &Backend) -> Result<(Vec<Repo>, Vec<Repo>), String>` — the registered repositories whose folders are there, and those whose folders are not found. Nothing is forgotten here: a folder can be away for a while. Blocks; run it off the UI thread.
- `type Reached = (Backend, Vec<Repo>, Vec<Repo>, Option<String>)` — A backend just reached, with its registered repositories (those whose folders are there, and those whose folders are not found) and the id of the folder the app was started on.
- `fn reach(attempt: connection::Attempt, register: bool) -> Result<Reached, String>` — reach a backend as `attempt` allows and list its repositories. With `register`, also register the folder the app was started on; failure there is not fatal, the user may have other repositories. Blocks; run it off the UI thread.
- `fn load_notes(backend: &Backend, id: &str) -> Result<(Value, Value), String>` — the notes of repository `id` and its proposals. Blocks; run it off the UI thread.
- `fn read_auto_explain() -> Option<cw_core::auto_explain::Count>` — how many notes auto explaining explains in a session, or `None` while it is off: `auto-explain.txt` and `auto-explain-count.txt` in the data directory, which the app reads itself, as it does the folders kept from agents.
- `recent_menu_entries` — A theme as a row of a menu: its icon in a color of its own, its name, a sample of its four colors, and a mark on the one in use. The toolbar's theme menu and the theme select of the settings list themes with it, so that a theme looks the same in both. What the toolbar's Open recent menu lists: the registered repositories of `recent`, newest first, by id and label, without the open one `current`, and at most as many as the tray's Recent menu.
- `enum ToolbarFit` — how much of the toolbar shows. Its buttons with labels need more room than a window about 1000 px wide leaves it, or one at 125% zoom: they ran past its right edge, and Sync, Review and To explain could not be reached.
- `const SEARCH_HINT: &str` — the search field's placeholder, whole and short.
- `fn main()` — private function.

And 42 more items.

## Depends on

- [[Review Module]] — uses `Click`, `Item`, `State`, `Targets`
- [[Live Module]] — uses `HomeInput`, `Kept`, `OPEN`, `Project`
- [[Graph Module (codewiki-app)]] — uses `Camera`, `DETAIL_LIMIT`, `GraphData`, `cell_size`
- [[Doc View Module]] — uses `Controls`, `DocPage`, `OnAction`, `PageSlot`
- [[Menu Module]] — uses `AnchorBounds`, `Menu`, `MenuItem`, `MenuSection`
- [[Settings View Module]] — uses `JOB_PAUSE`, `LOG_LEVEL_PREF`, `OpenMenu`, `State`
- [[Supervisor]] — uses `Supervisor`, `default`, `due`, `failed`
- [[Glass Module]] — uses `FADE`, `Levels`, `OPAQUE`, `chrome`

## Used by

- [[Review View Module]] — from `empty_review`
- [[Settings View Module]] — from `new`

## Gotchas

- `zoom::init` must come after `themes::init`, so the theme is still at 100% when zoom takes its sizes as the base.
- Hiding and showing the window call back into GPUI. They run only in `listen`, never inside an update. The close handler therefore sends `Event::Hide` and does not hide the window itself.
- A release build has no console. Logs and panics go to the app log file, so look there.
- The window's minimum size and first bounds are in screen pixels and do not change with zoom.
- Ctrl+Shift+= and the keypad plus arrive as "+", so zoom in has two bindings.
- A launch with the tray option never shows its window until something asks for it; `placed` stops the app from showing a window GPUI has not yet positioned.
- The backend is asked to stop with a short timeout, because GPUI waits only 200 ms for the quit hook.

## See also

- [[_index]]
- [[Crate Dependency Flow]]
