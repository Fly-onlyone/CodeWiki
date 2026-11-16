//! Starting the app when the user signs in to Windows.
//!
//! Windows runs what the values under the user's `Run` key say. The app's
//! value is its program with `--tray`, so that it starts in the tray, with no
//! window on screen. That value is the whole setting: there is no file
//! beside it that could disagree, and whoever removes it by hand (or with
//! `uninstall.ps1`) has turned the setting off.
//!
//! Only an installed copy offers this. A build from the sources would leave
//! behind a value that names a program in `target\`, which the next build
//! replaces or a cleanup deletes.
//!
//! Windows has a second switch, "Startup apps" in its settings and in the
//! Task Manager, which keeps a program from starting without touching this
//! value. The app does not read it: off there, the setting here still says
//! on.

use std::path::Path;

use windows_registry::CURRENT_USER;

/// The key whose values Windows runs at sign-in, under the user's hive.
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
/// The name of the app's value.
const VALUE: &str = "CodeWiki";
/// The program's file name in an installation.
const PROGRAM: &str = "codewiki-app.exe";

/// What Windows is to run for the installation in `install`: the program,
/// quoted because the folder may hold a space, started for the tray.
pub fn command(install: &Path) -> String {
    format!("\"{}\" --tray", install.join(PROGRAM).display())
}

/// Whether `value`, as read from the `Run` key, starts the installation in
/// `install`. Windows reads paths without regard to case. A value that
/// starts another copy is not this installation's setting.
pub fn matches(value: Option<&str>, install: &Path) -> bool {
    value.is_some_and(|value| value.trim().eq_ignore_ascii_case(&command(install)))
}

/// Whether Windows starts this installation at sign-in. `None` for a copy
/// that is not installed, which cannot be started this way.
pub fn enabled() -> Option<bool> {
    let install = cw_update::installed_dir()?;
    Some(matches(read(RUN_KEY).as_deref(), &install))
}

/// Turn the start at sign-in on or off for this installation.
pub fn set(on: bool) -> Result<(), String> {
    let install = cw_update::installed_dir()
        .ok_or("Only an installed copy of CodeWiki can start with Windows.")?;
    if on {
        write(RUN_KEY, &command(&install))
    } else {
        remove(RUN_KEY)
    }
}

fn read(key: &str) -> Option<String> {
    CURRENT_USER.open(key).ok()?.get_string(VALUE).ok()
}

fn write(key: &str, command: &str) -> Result<(), String> {
    CURRENT_USER
        .create(key)
        .and_then(|key| key.set_string(VALUE, command))
        .map_err(|e| format!("Windows did not take the startup setting: {e}"))
}

fn remove(key: &str) -> Result<(), String> {
    if read(key).is_none() {
        return Ok(());
    }
    CURRENT_USER
        .create(key)
        .and_then(|key| key.remove_value(VALUE))
        .map_err(|e| format!("Windows did not drop the startup setting: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_starts_the_installed_program_for_the_tray() {
        let install = Path::new(r"C:\Users\Software Tester\AppData\Local\Programs\CodeWiki");
        assert_eq!(
            command(install),
            r#""C:\Users\Software Tester\AppData\Local\Programs\CodeWiki\codewiki-app.exe" --tray"#
        );
    }

    #[test]
    fn only_a_value_that_starts_this_installation_counts() {
        let install = Path::new(r"C:\Programs\CodeWiki");
        assert!(matches(Some(&command(install)), install));
        // Windows reads paths without regard to case, and a value typed by
        // hand may carry a space at its end.
        assert!(matches(
            Some(r#""c:\programs\codewiki\CODEWIKI-APP.EXE" --tray "#),
            install
        ));
        // Another copy, or the program without the flag, is not it.
        assert!(!matches(
            Some(r#""D:\Other\CodeWiki\codewiki-app.exe" --tray"#),
            install
        ));
        assert!(!matches(
            Some(r#""C:\Programs\CodeWiki\codewiki-app.exe""#),
            install
        ));
        assert!(!matches(Some(""), install));
        assert!(!matches(None, install));
    }

    #[test]
    fn the_setting_is_one_value_written_read_and_removed() {
        // A key of the test's own, never the one Windows runs.
        let key = format!(r"Software\CodeWiki-test\autostart-{}", std::process::id());
        assert_eq!(read(&key), None);
        // Removing what is not there is no error.
        assert_eq!(remove(&key), Ok(()));

        let command = command(Path::new(r"C:\Programs\CodeWiki"));
        assert_eq!(write(&key, &command), Ok(()));
        assert_eq!(read(&key).as_deref(), Some(command.as_str()));

        assert_eq!(remove(&key), Ok(()));
        assert_eq!(read(&key), None);
        CURRENT_USER.remove_tree(&key).unwrap();
        // Leave nothing of the test behind, if it was the only one.
        if CURRENT_USER
            .open(r"Software\CodeWiki-test")
            .and_then(|key| key.keys().map(|mut keys| keys.next().is_none()))
            .unwrap_or(false)
        {
            CURRENT_USER.remove_tree(r"Software\CodeWiki-test").unwrap();
        }
    }
}
