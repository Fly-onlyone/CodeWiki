//! The switch for auto explaining.
//!
//! When it is on, coding agents explain the waiting wiki notes on their own at
//! the start of a session. One small text file in the data directory holds the
//! switch. `codewiki hook session-start` and the desktop app read it; the app
//! changes it. It is on by default. A file that cannot be read or understood
//! means off, so a broken file never spends anyone's allowance.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Name of the switch file inside the data directory.
pub const FILE: &str = "auto-explain.txt";

/// Path of the switch file.
pub fn file(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE)
}

/// Whether auto explaining is on. A missing file means on. The file holds `on`
/// or `off`; anything else, or a file that cannot be read, means off.
pub fn load(data_dir: &Path) -> bool {
    match std::fs::read_to_string(file(data_dir)) {
        Ok(text) => text.trim().eq_ignore_ascii_case("on"),
        Err(e) => e.kind() == std::io::ErrorKind::NotFound,
    }
}

/// Turn auto explaining on or off. Creates the data directory when it is
/// missing.
pub fn save(data_dir: &Path, on: bool) -> Result<()> {
    let path = file(data_dir);
    std::fs::create_dir_all(data_dir).map_err(|e| Error::io(data_dir, e))?;
    let draft = data_dir.join(format!("{FILE}.tmp"));
    let text = if on { "on\n" } else { "off\n" };
    std::fs::write(&draft, text).map_err(|e| Error::io(&draft, e))?;
    std::fs::rename(&draft, &path).map_err(|e| Error::io(&path, e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_on() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load(dir.path()));
    }

    #[test]
    fn off_is_off_and_on_is_on() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(file(dir.path()), "off").unwrap();
        assert!(!load(dir.path()));
        std::fs::write(file(dir.path()), "on\n").unwrap();
        assert!(load(dir.path()));
    }

    #[test]
    fn anything_else_is_off() {
        for text in ["", "yes", "true", "1", "onn", "{}"] {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(file(dir.path()), text).unwrap();
            assert!(!load(dir.path()), "{text:?}");
        }
    }

    #[test]
    fn an_unreadable_file_is_off() {
        let dir = tempfile::tempdir().unwrap();
        // A folder where the file should be cannot be read as text.
        std::fs::create_dir(file(dir.path())).unwrap();
        assert!(!load(dir.path()));
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), false).unwrap();
        assert!(!load(dir.path()));
        assert_eq!(std::fs::read_to_string(file(dir.path())).unwrap(), "off\n");
        save(dir.path(), true).unwrap();
        assert!(load(dir.path()));
        assert_eq!(std::fs::read_to_string(file(dir.path())).unwrap(), "on\n");
    }

    #[test]
    fn save_creates_a_missing_folder() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b");
        save(&nested, false).unwrap();
        assert!(!load(&nested));
    }
}
