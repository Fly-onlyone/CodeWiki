---
tags: [core, codewiki-app, module]
---

# Win32 Module

> The Win32 calls GPUI has no way to make.

## What it does

This is the only module of the workspace with `unsafe` code: keep every such call here, behind a safe function that says what it needs.

`show` and `hide` make Windows call back into GPUI before they return (it tells the window that it is shown or hidden, and may ask it to draw). Call them only from where GPUI is not in the middle of something: the app's event task, between two updates, never inside one.

## Source

- `crates/codewiki-app/src/win32.rs` — module `codewiki_app::win32`

## How it works

1. `AppView::listen` and `AppView::start_tray` in [[codewiki-app]] call `hwnd`.
2. `AppView::listen` in [[codewiki-app]] calls `hide`.
3. `AppView::listen` in [[codewiki-app]] calls `show`.
4. `AppView::listen` in [[codewiki-app]] calls `is_visible`.
5. `Tray::new` in [[Tray Module]] calls `keep_open`.

4 more entry points: `small_icon_size`, `wheel_scroll_lines`, `allow_foreground`, `on_console_stop`.

## Items

- `pub struct Hwnd` — A window of this process, as Windows names it. Only `hwnd` makes one, from a window GPUI has open, so the name is that of a real window for as long as that window lives. The app's one window lives as long as the app.
- `pub fn hwnd(window: &Window) -> Option<Hwnd>` — what Windows calls `window`.
- `pub fn hide(window: Hwnd)` — take the window off the screen and off the taskbar. It keeps everything: its place, its page, what was typed.
- `pub fn show(window: Hwnd)` — put a window that `hide` took away back where it was. It does not bring it to the front; GPUI's `activate_window` does.
- `pub fn is_visible(window: Hwnd) -> bool` — whether the window is on screen, minimized or not; false once hidden.
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

- [[codewiki-app]] — from `listen`, `main`, `start_tray`
- [[Backdrop Module]] — from `render`
- [[Instance Module]] — from `hand_over`
- [[Tray Module]] — from `new`

## See also

- [[_index]]
- [[codewiki-app]]
