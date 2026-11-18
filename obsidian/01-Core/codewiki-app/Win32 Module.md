---
tags: [core, codewiki-app, module]
---

# Win32 Module

> The Win32 calls GPUI has no way to make.

## Source

- `crates/codewiki-app/src/win32.rs` — module `codewiki_app::win32`

## How it works

This is the only module of the workspace with `unsafe` code: keep every such call here, behind a safe function that says what it needs.

Main items:

- `Hwnd` — A window of this process, as Windows names it.
- `hwnd` — what Windows calls `window`.
- `allow_foreground` — let the process `pid` bring one of its windows to the front.
- `small_icon_size` — the side of a small icon, as in the notification area, in pixels on the main screen.
- `hide` — take the window off the screen and off the taskbar.
- `is_visible` — whether the window is on screen, minimized or not; false once hidden.
- `show` — put a window that `hide` took away back where it was.
- `keep_open` — make the window behind the tray icon ignore a request to close.

## Depends on

- [[Diagrams Module]] — uses `store`
- [[Icons Module]] — uses `load`

## Used by

- [[codewiki-app]] — from `listen`, `start_tray`
- [[Instance Module]] — from `hand_over`
- [[Tray Module]] — from `new`

## See also

- [[_index]]
- [[codewiki-app]]
