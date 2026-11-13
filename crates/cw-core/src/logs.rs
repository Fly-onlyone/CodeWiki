//! Log files of the programs, under `<data dir>/logs`.
//!
//! The desktop app has no console, and the backend it starts has none either,
//! so what they report would otherwise be lost. Each program appends to its
//! own file. A file that grows past [`LIMIT`] is moved aside to
//! `<name>.old.log`, replacing the one before, so a program never keeps more
//! than two.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::paths::data_dir;

/// Largest a log file grows before it is moved aside.
pub const LIMIT: u64 = 1024 * 1024;

/// The log file of program `name`: `<data dir>/logs/<name>.log`.
pub fn path(name: &str) -> PathBuf {
    data_dir().join("logs").join(format!("{name}.log"))
}

/// Open the log at `path` for appending, creating its folder, after moving it
/// aside when it is over [`LIMIT`].
pub fn open(path: &Path) -> io::Result<File> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    rotate(path);
    OpenOptions::new().create(true).append(true).open(path)
}

/// Move `path` to `<name>.old.log` beside it when it is over [`LIMIT`].
/// Returns whether it was moved. The file must not be open in this process,
/// or Windows refuses the move.
pub fn rotate(path: &Path) -> bool {
    if !fs::metadata(path).is_ok_and(|meta| meta.len() > LIMIT) {
        return false;
    }
    let old = path.with_extension("old.log");
    let _ = fs::remove_file(&old);
    fs::rename(path, &old).is_ok()
}

/// `time` in UTC as `2026-11-13T20:03:13.123Z`, the form every log line
/// starts with.
pub fn timestamp(time: SystemTime) -> String {
    let since = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let seconds = since.as_secs();
    let (year, month, day) = civil_date((seconds / 86_400) as i64);
    let of_day = seconds % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        of_day / 3600,
        of_day / 60 % 60,
        of_day % 60,
        since.subsec_millis()
    )
}

/// The calendar date `days` days after 1970-01-01, in the proleptic
/// Gregorian calendar (Howard Hinnant's `civil_from_days`).
fn civil_date(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;
    use std::time::Duration;

    use super::*;

    /// A fresh folder under the system's temporary folder, removed when the
    /// test ends.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(test: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("codewiki-logs-{test}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn dates_are_counted_from_1970() {
        assert_eq!(civil_date(0), (1970, 1, 1));
        assert_eq!(civil_date(-1), (1969, 12, 31));
        // A leap day, and the first day of a year.
        assert_eq!(civil_date(11_016), (2000, 2, 29));
        assert_eq!(civil_date(19_723), (2024, 1, 1));
        assert_eq!(civil_date(20_405), (2025, 11, 13));
    }

    #[test]
    fn a_timestamp_is_utc_to_the_millisecond() {
        let time = UNIX_EPOCH + Duration::from_millis(1_704_067_200_000 + 3_723_045);
        assert_eq!(timestamp(time), "2024-01-01T01:02:03.045Z");
        assert_eq!(timestamp(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn opening_creates_the_folder_and_appends() {
        let scratch = Scratch::new("append");
        let log = scratch.0.join("logs").join("app.log");
        writeln!(open(&log).unwrap(), "one").unwrap();
        writeln!(open(&log).unwrap(), "two").unwrap();
        assert_eq!(fs::read_to_string(&log).unwrap(), "one\ntwo\n");
    }

    #[test]
    fn a_full_log_is_moved_aside_and_the_older_one_dropped() {
        let scratch = Scratch::new("rotate");
        fs::create_dir_all(&scratch.0).unwrap();
        let log = scratch.0.join("backend.log");
        let old = scratch.0.join("backend.old.log");
        fs::write(&old, "oldest").unwrap();
        fs::write(&log, vec![b'x'; LIMIT as usize + 1]).unwrap();
        writeln!(open(&log).unwrap(), "fresh").unwrap();
        assert_eq!(fs::read_to_string(&log).unwrap(), "fresh\n");
        assert_eq!(fs::metadata(&old).unwrap().len(), LIMIT + 1);
    }

    #[test]
    fn a_log_under_the_limit_stays_where_it_is() {
        let scratch = Scratch::new("small");
        fs::create_dir_all(&scratch.0).unwrap();
        let log = scratch.0.join("app.log");
        fs::write(&log, "small").unwrap();
        assert!(!rotate(&log));
        assert!(!scratch.0.join("app.old.log").exists());
    }

    #[test]
    fn each_program_has_its_own_file_in_the_logs_folder() {
        let app = path("app");
        assert_eq!(app.file_name().unwrap(), "app.log");
        assert_eq!(app.parent().unwrap().file_name().unwrap(), "logs");
        assert_ne!(path("app"), path("backend"));
    }
}
