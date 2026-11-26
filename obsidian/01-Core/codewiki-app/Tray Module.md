---
tags: [core, codewiki-app, module]
---

# Tray Module

> The app's icon in the notification area, with its tooltip and its menu.

## What it does

The app keeps running when its window closes; this icon is then all of it that shows. A click brings the window back, a right click opens the menu.

What the tooltip and the menu say is a `Snapshot` of the app, and `rows` is the menu for one, as plain data. `Tray::refresh` changes only the rows that differ from the last snapshot, in place: the menu may be open while the app's state changes, and a menu that is rebuilt while open closes under the pointer.

Clicks arrive in a window procedure, where nothing of GPUI may be touched. The two callbacks given to `Tray::new` must therefore only pass them on (the app sends them through its event channel).

## Source

- `crates/codewiki-app/src/tray.rs` — module `codewiki_app::tray`

## How it works

The app builds a `Snapshot` (open repository, state words, MCP words, whether sync can run, recent folders, a newer version if found). `rows` turns it into the menu as plain data: three labels, a version or update row, Open, Update wiki now, Recent repositories, Settings, Check for updates, Quit.

`Tray::new` creates the real menu from those rows, loads the icon from the program's resources, sets the tooltip, and installs two handlers. A left click or double click (`shows_window`) calls `on_show`. A menu row calls `on_command` with the `Command` decoded from the row's id. Right click opens the menu by itself.

`Tray::refresh` compares new rows with the old ones and changes only rows that differ: text, enabled state, or the recent submenu's items. The tooltip is updated only when it changes. `menu_text` doubles `&` so Windows does not read it as an underline marker. `recent_label` shows a folder's name, then its parent in brackets.

## Items

- `const ICON_RESOURCE: u16` — the icon in the program's resources (`resources/codewiki.rc`).
- `pub const RECENT_ROWS: usize` — how many repositories "Recent repositories" lists.
- `pub enum Command` — what a row of the menu asks the app to do.
- `pub struct Snapshot` — what the tray shows of the app, at one moment.
- `pub enum Row` — one row of the menu.
- `pub fn rows(snapshot: &Snapshot) -> Vec<Row>` — the menu for `snapshot`, top to bottom. The rows are always the same, in the same order; only their words, whether they can be chosen and the folders under "Recent repositories" change. `Tray::refresh` relies on it.
- `pub fn tooltip(snapshot: &Snapshot) -> String` — the tooltip for `snapshot`.
- `fn recent_label(path: &str) -> String` — how a folder reads under "Recent repositories": its name, then where it is, since two folders may share a name.
- `fn menu_text(text: &str) -> String` — `text` as a menu shows it: Windows reads `&` as "underline the next letter", so one that is meant is doubled.
- `enum Item` — the row of the real menu that stands for a `Row`.
- `pub struct Tray` — the icon, for as long as it lives; dropping it takes the icon away.
- `fn fill_recent(submenu: &Submenu, folders: &[(Command, String)]) -> Result<(), String>` — private function, returns `Result<(), String>`; called by `Tray::new`, `Tray::refresh`.
- `fn shows_window(event: &TrayIconEvent) -> bool` — whether a click on the icon asks for the window: the left button, let go, or a double click with it. The right button opens the menu by itself.

## Depends on

- [[cw-core]] — uses `APP_NAME`, `VERSION`
- [[Status Module]] — uses `tooltip`
- [[Win32 Module]] — uses `keep_open`

## Used by

- [[codewiki-app]] — from `AppView`, `Event`, `run_tray_command`, `start_tray`

## Gotchas

- The menu must keep the same rows in the same order. `refresh` pairs old and new rows by position. For example, with no update the version row is still there, greyed.
- Do not rebuild the menu while it may be open: a rebuilt open menu closes under the pointer.
- The callbacks run in a window procedure. They must only pass the event on, never touch GPUI.
- Only the first event handler set in a process is kept. The app has exactly one tray icon.
- `keep_open` is called because, after the menu was open, the icon's window has the keyboard and Alt+F4 would destroy it and remove the icon.
- Quit is the last row so a script can reach it with Up and Enter.

## See also

- [[_index]]
- [[codewiki-app]]
