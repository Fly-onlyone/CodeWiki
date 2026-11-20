//! The Win32 calls GPUI has no way to make. This is the only module of the
//! workspace with `unsafe` code: keep every such call here, behind a safe
//! function that says what it needs.
//!
//! [`show`] and [`hide`] make Windows call back into GPUI before they
//! return (it tells the window that it is shown or hidden, and may ask it to
//! draw). Call them only from where GPUI is not in the middle of something:
//! the app's event task, between two updates, never inside one.

#![allow(unsafe_code)]

use std::sync::OnceLock;
use std::sync::atomic::{AtomicIsize, Ordering};

use gpui_kit::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT, SetConsoleCtrlHandler};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, CallWindowProcW, DefWindowProcW, GWLP_WNDPROC, GetSystemMetrics,
    IsWindowVisible, SM_CXSMICON, SPI_GETWHEELSCROLLLINES, SW_HIDE, SW_SHOW, SetWindowLongPtrW,
    ShowWindow, SystemParametersInfoW, WM_CLOSE, WNDPROC,
};

/// A window of this process, as Windows names it. Only [`hwnd`] makes one,
/// from a window GPUI has open, so the name is that of a real window for as
/// long as that window lives. The app's one window lives as long as the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hwnd(isize);

impl Hwnd {
    fn raw(self) -> HWND {
        self.0 as HWND
    }
}

/// What Windows calls `window`.
pub fn hwnd(window: &Window) -> Option<Hwnd> {
    // `Window` has a method of its own with this name, which returns GPUI's
    // handle; the trait's is the one that names the system's window.
    let handle = HasWindowHandle::window_handle(window).ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Some(Hwnd(handle.hwnd.get())),
        _ => None,
    }
}

/// Take the window off the screen and off the taskbar. It keeps everything:
/// its place, its page, what was typed.
pub fn hide(window: Hwnd) {
    // SAFETY: `window` names a window of this process (see `Hwnd`). The call
    // changes whether it shows and touches no memory of ours.
    unsafe { ShowWindow(window.raw(), SW_HIDE) };
}

/// Put a window that [`hide`] took away back where it was. It does not
/// bring it to the front; GPUI's `activate_window` does.
pub fn show(window: Hwnd) {
    // SAFETY: as for `hide`.
    unsafe { ShowWindow(window.raw(), SW_SHOW) };
}

/// Whether the window is on screen, minimized or not; false once hidden.
pub fn is_visible(window: Hwnd) -> bool {
    // SAFETY: as for `hide`; the call only reads the window's state.
    unsafe { IsWindowVisible(window.raw()) != 0 }
}

/// The window procedure the tray icon's window had before [`keep_open`]
/// replaced it; 0 until then.
static TRAY_PROCEDURE: AtomicIsize = AtomicIsize::new(0);

/// Make the window behind the tray icon ignore a request to close.
///
/// The tray icon lives in a window of its own that never shows. Opening the
/// icon's menu makes that window the one that has the keyboard, and it stays
/// so after the menu closes. Alt+F4 then goes to it. Windows' own answer to
/// that is to destroy the window, and the icon went with it while the app
/// ran on, with nothing left on screen to reach it by. The window must end
/// only when the icon is dropped, which destroys it directly and asks nobody.
///
/// `window` is what `TrayIcon::window_handle` returns: a window of this
/// process that lives as long as the icon. Call this once, from the thread
/// that made the icon, before that thread handles another message.
pub fn keep_open(window: HWND) {
    let replacement: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT =
        tray_procedure;
    // SAFETY: `window` is a live window of this process (see above), and
    // `tray_procedure` has the signature Windows asks of a window procedure.
    // The call returns the procedure it replaces. No message reaches the new
    // one before that is stored: messages for the window are handled on this
    // thread, which handles none until this function has returned.
    let previous =
        unsafe { SetWindowLongPtrW(window, GWLP_WNDPROC, replacement as usize as isize) };
    if previous == 0 {
        log::warn!("the tray icon's window keeps its own answer to a close request");
        return;
    }
    TRAY_PROCEDURE.store(previous, Ordering::Relaxed);
}

/// The tray window's procedure after [`keep_open`]: a close request is
/// answered as handled and does nothing; every other message goes to the
/// procedure the window had before.
unsafe extern "system" fn tray_procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_CLOSE {
        return 0;
    }
    let previous = TRAY_PROCEDURE.load(Ordering::Relaxed);
    // SAFETY: Windows calls this for the tray icon's window with a message
    // meant for it. `previous` is 0, or the value `SetWindowLongPtrW`
    // returned for that window's procedure in `keep_open`, which is a window
    // procedure by that function's contract; `WNDPROC` is an optional
    // pointer to one and has the size of `isize`. Each call passes the
    // message on unchanged.
    unsafe {
        match std::mem::transmute::<isize, WNDPROC>(previous) {
            Some(procedure) => CallWindowProcW(Some(procedure), window, message, wparam, lparam),
            None => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}

/// The side of a small icon, as in the notification area, in pixels on the
/// main screen.
pub fn small_icon_size() -> u32 {
    // SAFETY: the call reads one system setting and takes no pointer.
    let side = unsafe { GetSystemMetrics(SM_CXSMICON) };
    // 0 means the call failed; 16 is the size at 100% scaling.
    u32::try_from(side)
        .ok()
        .filter(|side| *side > 0)
        .unwrap_or(16)
}

/// How many lines one notch of the mouse wheel scrolls, as set in Windows.
/// GPUI reports a notch as that many lines; zoom counts notches.
pub fn wheel_scroll_lines() -> u32 {
    // Windows' default, kept should the call fail.
    let mut lines: u32 = 3;
    // SAFETY: for SPI_GETWHEELSCROLLLINES Windows writes one u32 through the
    // pointer, which points at `lines`, alive and writable for the call. The
    // last argument is 0: nothing is written to the user's profile.
    let read =
        unsafe { SystemParametersInfoW(SPI_GETWHEELSCROLLLINES, 0, (&raw mut lines).cast(), 0) };
    if read == 0 { 3 } else { lines }
}

/// Let the process `pid` bring one of its windows to the front.
///
/// Windows lets only the process the user is working with take the
/// foreground. A second launch of the app is that process: the user has just
/// started it. It hands its request to the app that is already running, and
/// that one, in the background until now, could only flash its taskbar
/// button. The second launch calls this first, with the running app's id.
/// It fails when this process may not take the foreground either; the window
/// then flashes, which is all that can be done.
pub fn allow_foreground(pid: u32) {
    // SAFETY: the call takes a process id by value and reads no memory of
    // ours; an id that names no process makes it fail, which is reported.
    let allowed = unsafe { AllowSetForegroundWindow(pid) };
    if allowed == 0 {
        log::info!("could not let process {pid} take the foreground");
    }
}

/// What [`on_console_stop`] was given; unset until then.
static CONSOLE_STOP: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

/// Call `stop` when the console the app runs in sends Ctrl+C or Ctrl+Break,
/// as an IDE's Stop button does, instead of letting Windows end the process.
///
/// Windows' own answer ends the process at once, with exit code 0xc000013a.
/// That skips `on_app_quit`, so the backend is never asked to stop: started
/// with no console, it never sees the Ctrl+C and runs on, serving the old
/// build and keeping its file locked. `stop` runs on a thread Windows makes
/// for the event; it should hand the request over and return. A release
/// build has no console, so there the handler never runs. The first call
/// counts; later ones do nothing.
pub fn on_console_stop(stop: impl Fn() + Send + Sync + 'static) {
    if CONSOLE_STOP.set(Box::new(stop)).is_err() {
        return;
    }
    // SAFETY: `console_handler` has the signature Windows asks of a console
    // control handler and is a function of this program, alive as long as
    // the process. The call takes no other pointer.
    if unsafe { SetConsoleCtrlHandler(Some(console_handler), 1) } == 0 {
        log::warn!("Ctrl+C in the console will end the app without stopping the backend");
    }
}

/// The console control handler [`on_console_stop`] adds: Ctrl+C and
/// Ctrl+Break go to its `stop` and count as handled; every other event
/// (the console closing, logoff, shutdown) goes on to Windows' own answer.
unsafe extern "system" fn console_handler(event: u32) -> windows_sys::core::BOOL {
    match event {
        CTRL_C_EVENT | CTRL_BREAK_EVENT => {
            if let Some(stop) = CONSOLE_STOP.get() {
                stop();
            }
            1
        }
        _ => 0,
    }
}
