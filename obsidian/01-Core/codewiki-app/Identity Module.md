---
tags: [core, codewiki-app, module]
---

# Identity Module

> The name and the icon Windows shows above the app's notifications.

## What it does

A Windows notification shows a name and an icon in its header. For an app without a package, Windows takes both from the registry, under a key named by the app's id. GPUI writes the name only, so the header had no icon. This module fills that gap: it saves the app's logo as a PNG in the data folder and writes the name and the icon path to the registry key. It runs just before the app's first Windows notification.

## Source

- `crates/codewiki-app/src/identity.rs` — module `codewiki_app::identity`

## How it works

`AppView::show_system_notification` calls `register` once per process. `register` first calls `write_icon`, which writes `notification-icon.png` to the data folder unless the file already holds the same bytes. It then opens `Software\Classes\AppUserModelId\<id>` under the current user and calls `update`.

`update` asks `stale` which of the two wanted values (`DisplayName` and `IconUri`) the key lacks or holds differently, and writes only those. `read` reports both the text and the registry type, because the type counts too. `register` returns what it wrote, an empty list when all was in place, so a second run changes nothing.

## Items

- `pub const ICON_FILE: &str` — the file the logo is written to, in the data folder.
- `pub struct Text` — A string value of the key, as the registry holds it.
- `pub fn wanted(name: &str, icon: &Path) -> [(&'static str, Text); 2]` — the values the key must hold for an app named `name` whose icon is the file `icon`.
- `stale` — the values of `wanted` the key does not hold as they are: missing, or with other text, or of the other type. `current` reads a value by name.
- `pub fn register(id: &str, name: &str, logo: &[u8]) -> Result<Vec<&'static str>, String>` — have Windows show `name` and `logo` above the notifications of the app `id`. Returns what was written: the icon file, the names of the values, or nothing when all was in place.
- `fn write_icon(dir: &Path, logo: &[u8]) -> Result<(PathBuf, bool), String>` — write `logo` to `ICON_FILE` in `dir`, unless the file already holds it. Returns the file's full path, which `IconUri` needs, and whether it was written.
- `fn update(key: &Key, wanted: &[(&'static str, Text)]) -> Result<Vec<&'static str>, String>` — write the values of `wanted` that `key` does not hold as they are. Returns their names.
- `fn read(key: &Key, name: &str) -> Option<Text>` — the string value `name` of `key`; `None` when it is missing or not a string.

## Depends on

- [[Paths Module]] — uses `data_dir`

## Used by

- [[codewiki-app]] — from `show_system_notification`

## Gotchas

- `DisplayName` must be a plain string (`REG_SZ`), as GPUI writes it. `IconUri` is an expandable string (`REG_EXPAND_SZ`), as Microsoft's example writes it. A value of the wrong type counts as stale and is rewritten, so the two writers do not take turns.
- `IconUri` needs the icon's full path, so `write_icon` makes the path absolute. If the data folder moves, the value is stale and is written again.
- A failure only loses the icon. The caller logs a warning and the notification still shows.
- The registration happens at the first notification, not at start. The same id decides which taskbar button the window belongs to, so a run that never notifies keeps the button Windows gave it.
- `uninstall.ps1` removes the key and the icon file. Keep that script in step if the key path changes.

## See also

- [[_index]]
- [[codewiki-app]]
