---
tags: [core, codewiki-app, module]
---

# Identity Module

> The name and the icon Windows shows above the app's notifications.

## What it does

An app without a package is known to Windows by the id it gives its process (`APP_IDENTITY` in `main.rs`). The header of its notifications shows what the user's registry holds for that id, under `Software\Classes\AppUserModelId\<id>`: `DisplayName`, and `IconUri`, an image file named by its full path. GPUI writes the name alone, at its first notification, so the header had no icon. This module writes the app's logo to `notification-icon.png` in the data folder, and both values, just before the app's first Windows notification. Each is written only when it is missing or different. `uninstall.ps1` removes the key and the file.

The values are those of Microsoft's "Send a local app notification from other types of unpackaged apps" and of the Windows App SDK, which registers an unpackaged app the same way (`RegisterAssets` in `dev/AppNotifications/AppNotificationUtility.cpp`). `IconBackgroundColor` is optional, and the SDK leaves it out too.

## Source

- `crates/codewiki-app/src/identity.rs` — module `codewiki_app::identity`

## How it works

1. `AppView::show_system_notification` in [[codewiki-app]] calls `register`. `register` calls `write_icon`, then `data_dir` in [[Paths Module]], then `update`, then `wanted`. `update` calls `stale`, then `read`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

## See also

- [[_index]]
- [[codewiki-app]]
