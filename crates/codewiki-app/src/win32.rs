//! The Win32 calls GPUI has no way to make. This is the only module of the
//! workspace with `unsafe` code: keep every such call here, behind a safe
//! function that says what it needs.

#![allow(unsafe_code)]

use windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow;

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
