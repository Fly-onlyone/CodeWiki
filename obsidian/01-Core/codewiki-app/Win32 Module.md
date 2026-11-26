---
tags: [core, codewiki-app, module]
---

# Win32 Module

> The Win32 calls GPUI has no way to make.

## What it does

GPUI, the UI toolkit, cannot do a few things Windows needs: hide a window to the tray, keep the tray icon alive on Alt+F4, read the mouse wheel setting, let a second launch take the foreground, and catch Ctrl+C from an IDE's Stop button. This module does them with raw Win32 calls.

It is the only module of the workspace allowed to use `unsafe`. Each call sits behind a safe function, and each has a SAFETY comment that says what it needs.

## Source

- `crates/codewiki-app/src/win32.rs` — module `codewiki_app::win32`

## How it works

`hwnd` turns a GPUI `Window` into an `Hwnd`, a small wrapper that only this function can make. Every other window call takes an `Hwnd`, which is how the safe functions know the window is real. `hide`, `show`, `is_visible` and `is_minimized` call `ShowWindow`, `IsWindowVisible` and `IsIconic`.

`keep_open` swaps the window procedure of the tray icon's hidden window for `tray_procedure`. That procedure answers `WM_CLOSE` with 0 and does nothing, and passes every other message to the old procedure, which `keep_open` saved in `TRAY_PROCEDURE`.

`on_console_stop` stores a callback in `CONSOLE_STOP` and registers `console_handler`. On Ctrl+C or Ctrl+Break the handler calls the callback and reports the event as handled. Other events (console close, logoff, shutdown) go on to Windows.

`small_icon_size`, `wheel_scroll_lines` and `allow_foreground` read a system setting or pass a process id by value. The first two fall back to 16 and 3 when the call fails.

## Items

- `pub struct Hwnd` — A window of this process, as Windows names it. Only `hwnd` makes one, from a window GPUI has open, so the name is that of a real window for as long as that window lives. The app's one window lives as long as the app.
- `pub fn hwnd(window: &Window) -> Option<Hwnd>` — what Windows calls `window`.
- `pub fn hide(window: Hwnd)` — take the window off the screen and off the taskbar. It keeps everything: its place, its page, what was typed.
- `pub fn show(window: Hwnd)` — put a window that `hide` took away back where it was. It does not bring it to the front; GPUI's `activate_window` does.
- `pub fn is_visible(window: Hwnd) -> bool` — whether the window is on screen, minimized or not; false once hidden.
- `pub fn is_minimized(window: Hwnd) -> bool` — whether the window is minimized to the taskbar.
- `static TRAY_PROCEDURE: AtomicIsize` — the window procedure the tray icon's window had before `keep_open` replaced it; 0 until then.
- `pub fn keep_open(window: HWND)` — make the window behind the tray icon ignore a request to close. The tray icon lives in a window of its own that never shows. Opening the icon's menu makes that window the one that has the keyboard, and it stays so after the menu closes. Alt+F4 then goes to it. Windows' own answer to that is to destroy the window, and the icon went with it while the app ran on, with nothing left on screen to reach it by. The window must end only when the icon is dropped, which destroys it directly and asks nobody. `window` is what `TrayIcon::window_handle` returns: a window of this process that lives as long as the icon. Call this once, from the thread that made the icon, before that thread handles another message.
- `tray_procedure` — the tray window's procedure after `keep_open`: a close request is answered as handled and does nothing; every other message goes to the procedure the window had before.
- `pub fn small_icon_size() -> u32` — the side of a small icon, as in the notification area, in pixels on the main screen.
- `pub fn wheel_scroll_lines() -> u32` — how many lines one notch of the mouse wheel scrolls, as set in Windows. GPUI reports a notch as that many lines; zoom counts notches.
- `pub fn allow_foreground(pid: u32)` — let the process `pid` bring one of its windows to the front. Windows lets only the process the user is working with take the foreground. A second launch of the app is that process: the user has just started it. It hands its request to the app that is already running, and that one, in the background until now, could only flash its taskbar button. The second launch calls this first, with the running app's id. It fails when this process may not take the foreground either; the window then flashes, which is all that can be done.
- `static CONSOLE_STOP: OnceLock<Box<dyn Fn() + Send + Sync>>` — what `on_console_stop` was given; unset until then.
- `pub fn on_console_stop(stop: impl Fn() + Send + Sync + 'static)` — call `stop` when the console the app runs in sends Ctrl+C or Ctrl+Break, as an IDE's Stop button does, instead of letting Windows end the process. Windows' own answer ends the process at once, with exit code 0xc000013a. That skips `on_app_quit`, so the backend is never asked to stop: started with no console, it never sees the Ctrl+C and runs on, serving the old build and keeping its file locked. `stop` runs on a thread Windows makes for the event; it should hand the request over and return. A release build has no console, so there the handler never runs. The first call counts; later ones do nothing.
- `unsafe extern "system" fn console_handler(event: u32) -> windows_sys::core::BOOL` — the console control handler `on_console_stop` adds: Ctrl+C and Ctrl+Break go to its `stop` and count as handled; every other event (the console closing, logoff, shutdown) goes on to Windows' own answer.

## Depends on

- [[Diagrams Module]] — uses `store`
- [[Icons Module]] — uses `load`

## Used by

- [[codewiki-app]] — from `AppView`, `listen`, `main`, `new`
- [[Backdrop Module]] — from `render`
- [[Instance Module]] — from `hand_over`
- [[Tray Module]] — from `new`

## Gotchas

- `show` and `hide` make Windows call back into GPUI before they return. Call them from the app's event task between updates, never inside a GPUI update.
- `show` does not bring the window to the front. GPUI's `activate_window` does that.
- Call `keep_open` once, on the thread that made the tray icon, before that thread handles another message. The saved old procedure must be stored before any message reaches the new one.
- Without `keep_open`, Alt+F4 on the tray window destroys it and the icon vanishes while the app keeps running.
- `on_console_stop`: the first call wins; later calls do nothing. The callback runs on a thread Windows makes, so hand the request over and return. Release builds have no console, so it never fires there.
- Without it, Ctrl+C ends the process at once (exit code 0xc000013a), skips the quit hook, and leaves the backend running with its file locked.
- `allow_foreground` can fail. Then the window only flashes in the taskbar.

## See also

- [[_index]]
- [[codewiki-app]]
