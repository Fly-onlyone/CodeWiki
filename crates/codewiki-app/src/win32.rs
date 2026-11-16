//! The Win32 calls GPUI has no way to make. This is the only module of the
//! workspace with `unsafe` code: keep every such call here, behind a safe
//! function that says what it needs.
//!
//! [`show`] and [`hide`] make Windows call back into GPUI before they
//! return (it tells the window that it is shown or hidden, and may ask it to
//! draw). Call them only from where GPUI is not in the middle of something:
//! the app's event task, between two updates, never inside one.

#![allow(unsafe_code)]

use gpui_kit::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, GetSystemMetrics, IsWindowVisible, SM_CXSMICON, SW_HIDE, SW_SHOW,
    ShowWindow,
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
