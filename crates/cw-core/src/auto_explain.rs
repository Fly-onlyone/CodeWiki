//! The switch for auto explaining.
//!
//! When it is on, coding agents explain the waiting wiki notes on their own at
//! the start of a session. One small text file in the data directory holds the
//! switch. `codewiki hook session-start` and the desktop app read it; the app
//! changes it. It is on by default. A file that cannot be read or understood
//! means off, so a broken file never spends anyone's allowance.
//!
//! A second file holds how many notes a session explains: a number, or `all`.
//! It is 20 when the file is missing or cannot be understood.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Name of the switch file inside the data directory.
pub const FILE: &str = "auto-explain.txt";

/// Name of the file inside the data directory that holds the [`Count`].
pub const COUNT_FILE: &str = "auto-explain-count.txt";

/// How many waiting notes a session explains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Count {
    /// At most this many; never 0.
    Notes(usize),
    /// Every note that waits.
    All,
}

/// The count when the user chose none.
pub const DEFAULT: Count = Count::Notes(20);

/// The counts the app offers, in its order.
pub const PRESETS: [Count; 6] = [
    Count::Notes(5),
    Count::Notes(10),
    Count::Notes(20),
    Count::Notes(50),
    Count::Notes(100),
    Count::All,
];

impl Count {
    /// The count `text` names: a positive number or `all`.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.eq_ignore_ascii_case("all") {
            return Some(Self::All);
        }
        match text.parse::<usize>() {
            Ok(0) | Err(_) => None,
            Ok(n) => Some(Self::Notes(n)),
        }
    }

    /// How many of `waiting` notes a session explains.
    pub fn of(self, waiting: usize) -> usize {
        match self {
            Self::Notes(n) => n.min(waiting),
            Self::All => waiting,
        }
    }
}

/// Path of the switch file.
pub fn file(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE)
}

/// Path of the count file.
pub fn count_file(data_dir: &Path) -> PathBuf {
    data_dir.join(COUNT_FILE)
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
    write(data_dir, FILE, if on { "on\n" } else { "off\n" })
}

/// How many notes a session explains. A missing file, one that cannot be
/// read, and one that names no positive number or `all` mean [`DEFAULT`].
pub fn load_count(data_dir: &Path) -> Count {
    std::fs::read_to_string(count_file(data_dir))
        .ok()
        .and_then(|text| Count::parse(&text))
        .unwrap_or(DEFAULT)
}

/// Save how many notes a session explains. Creates the data directory when
/// it is missing.
pub fn save_count(data_dir: &Path, count: Count) -> Result<()> {
    let text = match count {
        Count::Notes(n) => format!("{}\n", n.max(1)),
        Count::All => "all\n".to_string(),
    };
    write(data_dir, COUNT_FILE, &text)
}

/// Write `text` to `name` in the data directory through a draft, so a reader
/// never sees half a file.
fn write(data_dir: &Path, name: &str, text: &str) -> Result<()> {
    let path = data_dir.join(name);
    std::fs::create_dir_all(data_dir).map_err(|e| Error::io(data_dir, e))?;
    let draft = data_dir.join(format!("{name}.tmp"));
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

    #[test]
    fn a_missing_count_is_the_default() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_count(dir.path()), DEFAULT);
        assert_eq!(DEFAULT, Count::Notes(20));
    }

    #[test]
    fn a_count_is_a_positive_number_or_all() {
        let dir = tempfile::tempdir().unwrap();
        for (text, count) in [
            ("50", Count::Notes(50)),
            (" 7\n", Count::Notes(7)),
            ("all", Count::All),
            ("ALL\n", Count::All),
        ] {
            std::fs::write(count_file(dir.path()), text).unwrap();
            assert_eq!(load_count(dir.path()), count, "{text:?}");
        }
    }

    #[test]
    fn a_count_that_cannot_be_understood_is_the_default() {
        for text in ["", "0", "-3", "x", "20 notes", "unlimited"] {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(count_file(dir.path()), text).unwrap();
            assert_eq!(load_count(dir.path()), DEFAULT, "{text:?}");
        }
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(count_file(dir.path())).unwrap();
        assert_eq!(load_count(dir.path()), DEFAULT);
    }

    #[test]
    fn save_count_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a");
        for count in PRESETS {
            save_count(&nested, count).unwrap();
            assert_eq!(load_count(&nested), count);
        }
        save_count(&nested, Count::All).unwrap();
        assert_eq!(
            std::fs::read_to_string(count_file(&nested)).unwrap(),
            "all\n"
        );
        // The switch is a file of its own and stays as it was.
        assert!(load(&nested));
    }

    #[test]
    fn a_count_caps_the_waiting_notes() {
        assert_eq!(Count::Notes(20).of(7), 7);
        assert_eq!(Count::Notes(50).of(218), 50);
        assert_eq!(Count::All.of(218), 218);
        assert_eq!(Count::All.of(0), 0);
    }
}
