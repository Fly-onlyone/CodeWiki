//! Small per-user settings, one value per file under `%APPDATA%\CodeWiki`.
//!
//! Each setting is a name and a line of text. A missing or unreadable file is
//! "not set"; a failed write is ignored, because losing a preference must
//! never stop the app.

use std::path::{Path, PathBuf};

/// The settings directory, or `None` when the system does not say where
/// application data goes.
pub fn dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|root| PathBuf::from(root).join("CodeWiki"))
}

/// The value of the setting `name`, trimmed. `None` when it is not set or is
/// empty.
pub fn read(name: &str) -> Option<String> {
    read_in(&dir()?, name)
}

/// Remember `value` for the setting `name`.
pub fn write(name: &str, value: &str) {
    if let Some(dir) = dir() {
        write_in(&dir, name, value);
    }
}

fn file(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.txt"))
}

fn read_in(dir: &Path, name: &str) -> Option<String> {
    let text = std::fs::read_to_string(file(dir, name)).ok()?;
    let value = text.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn write_in(dir: &Path, name: &str, value: &str) {
    if std::fs::create_dir_all(dir).is_ok() {
        let _ = std::fs::write(file(dir, name), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh directory under the system's temporary directory, removed when
    /// the test ends.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(test: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("codewiki-prefs-{test}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_written_value_reads_back() {
        let scratch = Scratch::new("roundtrip");
        write_in(&scratch.0, "theme", "CodeWiki Violet");
        assert_eq!(
            read_in(&scratch.0, "theme").as_deref(),
            Some("CodeWiki Violet")
        );
    }

    #[test]
    fn writing_creates_the_directory() {
        let scratch = Scratch::new("mkdir");
        let nested = scratch.0.join("a").join("b");
        write_in(&nested, "motion", "reduced");
        assert_eq!(read_in(&nested, "motion").as_deref(), Some("reduced"));
    }

    #[test]
    fn a_missing_setting_is_not_set() {
        let scratch = Scratch::new("missing");
        assert_eq!(read_in(&scratch.0, "theme"), None);
        write_in(&scratch.0, "theme", "x");
        assert_eq!(read_in(&scratch.0, "motion"), None);
    }

    #[test]
    fn surrounding_whitespace_is_dropped_and_blank_is_not_set() {
        let scratch = Scratch::new("trim");
        write_in(&scratch.0, "theme", "  Nord\r\n");
        assert_eq!(read_in(&scratch.0, "theme").as_deref(), Some("Nord"));
        write_in(&scratch.0, "theme", " \n");
        assert_eq!(read_in(&scratch.0, "theme"), None);
    }

    #[test]
    fn a_later_write_replaces_the_value() {
        let scratch = Scratch::new("replace");
        write_in(&scratch.0, "motion", "reduced");
        write_in(&scratch.0, "motion", "full");
        assert_eq!(read_in(&scratch.0, "motion").as_deref(), Some("full"));
    }

    #[test]
    fn settings_live_in_one_file_each_named_after_the_setting() {
        let scratch = Scratch::new("files");
        write_in(&scratch.0, "theme", "a");
        write_in(&scratch.0, "motion", "b");
        assert!(scratch.0.join("theme.txt").is_file());
        assert!(scratch.0.join("motion.txt").is_file());
    }
}
