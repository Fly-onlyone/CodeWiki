//! Log files of the programs, under `<data dir>/logs`.
//!
//! The desktop app has no console, and the backend it starts has none either,
//! so what they report would otherwise be lost. Each program appends to its
//! own file. A file that grows past [`LIMIT`] is moved aside to
//! `<name>.old.log`, replacing the one before, so a program never keeps more
//! than two.
//!
//! [`init`] sends a program's `log` records to its file and to standard
//! error, so that a program run from a terminal or an IDE shows them there
//! as well. `CODEWIKI_LOG_DIR` moves the folder (a development run puts it in
//! the repository); `CODEWIKI_LOG` sets the level.

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use log::{LevelFilter, Log, Metadata, Record};

use crate::paths::data_dir;

/// Largest a log file grows before it is moved aside.
pub const LIMIT: u64 = 1024 * 1024;

/// Environment variable naming the folder of the log files. A relative path
/// is taken from the working folder.
pub const LOG_DIR_ENV: &str = "CODEWIKI_LOG_DIR";

/// Environment variable setting the level: `error`, `warn`, `info`, `debug`,
/// `trace` or `off`.
pub const LEVEL_ENV: &str = "CODEWIKI_LOG";

/// The folder of the log files: `CODEWIKI_LOG_DIR` when set, otherwise
/// `<data dir>/logs`.
pub fn dir() -> PathBuf {
    dir_from(std::env::var_os(LOG_DIR_ENV))
}

fn dir_from(value: Option<OsString>) -> PathBuf {
    match value.filter(|value| !value.is_empty()) {
        Some(value) => {
            let dir = PathBuf::from(value);
            // A program this one starts may run in another folder.
            std::path::absolute(&dir).unwrap_or(dir)
        }
        None => data_dir().join("logs"),
    }
}

/// The log file of program `name`: `<log dir>/<name>.log`.
pub fn path(name: &str) -> PathBuf {
    dir().join(format!("{name}.log"))
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

/// The length of a log file at which the next move aside is tried. A file
/// still over [`LIMIT`] after a try was held open by another process, which
/// Windows does not let move; reopening it for every line would only repeat
/// the failure, so the next try waits for another [`LIMIT`] of growth.
fn next_rotation(len: u64) -> u64 {
    if len > LIMIT {
        len.saturating_add(LIMIT)
    } else {
        LIMIT
    }
}

/// The open file, how long it is, and when to try moving it aside.
struct Sink {
    file: File,
    len: u64,
    rotate_at: u64,
}

impl Sink {
    fn open(path: &Path) -> Option<Self> {
        let file = open(path).ok()?;
        let len = file.metadata().map(|meta| meta.len()).unwrap_or(0);
        Some(Self {
            file,
            len,
            rotate_at: next_rotation(len),
        })
    }
}

/// A `log` logger that appends to a file and echoes to standard error.
struct Logger {
    /// `None` for a logger that only prints.
    path: Option<PathBuf>,
    level: LevelFilter,
    console: bool,
    /// `None` after a failed reopen; file records are then dropped.
    sink: Mutex<Option<Sink>>,
}

impl Logger {
    fn new(path: Option<PathBuf>, level: LevelFilter, console: bool) -> Self {
        let sink = path.as_deref().and_then(Sink::open);
        Self {
            path,
            level,
            console,
            sink: Mutex::new(sink),
        }
    }

    fn write_file(&self, line: &str) {
        let Some(path) = &self.path else {
            return;
        };
        // A panic while the lock was held must not silence the log.
        let mut sink = self
            .sink
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if sink.as_ref().is_some_and(|s| s.len > s.rotate_at) {
            // Close the file first: Windows will not move an open one.
            *sink = None;
            *sink = Sink::open(path);
        }
        if let Some(sink) = sink.as_mut()
            && sink.file.write_all(line.as_bytes()).is_ok()
        {
            sink.len += line.len() as u64;
        }
    }
}

/// One log line: time, level, where it came from, and the message.
fn line(time: SystemTime, record: &Record) -> String {
    format!(
        "{} {:<5} {}: {}\n",
        timestamp(time),
        record.level(),
        record.target(),
        record.args()
    )
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = line(SystemTime::now(), record);
        if self.console {
            // No console (a release build of the app) or a closed pipe: the
            // file still has the line.
            let _ = io::stderr().write_all(line.as_bytes());
        }
        self.write_file(&line);
    }

    fn flush(&self) {}
}

/// The level named by `value` (the value of `CODEWIKI_LOG`), or `default`
/// when it is unset or unknown.
pub fn level_from(value: Option<&str>, default: LevelFilter) -> LevelFilter {
    match value.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
        Some("trace") => LevelFilter::Trace,
        Some("debug") => LevelFilter::Debug,
        Some("info") => LevelFilter::Info,
        Some("warn") => LevelFilter::Warn,
        Some("error") => LevelFilter::Error,
        Some("off") => LevelFilter::Off,
        _ => default,
    }
}

fn level(default: LevelFilter) -> LevelFilter {
    level_from(std::env::var(LEVEL_ENV).ok().as_deref(), default)
}

/// Install `logger` as the process logger. False when one is set already.
fn install(logger: Logger) -> bool {
    let level = logger.level;
    if log::set_boxed_logger(Box::new(logger)).is_err() {
        return false;
    }
    log::set_max_level(level);
    true
}

/// Send the `log` records of program `name` at `default` and above (or at
/// the level in `CODEWIKI_LOG`) to `<log dir>/<name>.log` and to standard
/// error, and log every panic. Call first in `main`. Returns the file's path,
/// or `None` when the folder is not writable (the records then go to
/// standard error only) or a logger was installed already.
pub fn init(name: &str, default: LevelFilter) -> Option<PathBuf> {
    let path = path(name);
    let logger = Logger::new(Some(path.clone()), level(default), true);
    let writing = logger.sink.lock().is_ok_and(|sink| sink.is_some());
    if !install(logger) {
        return None;
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!(target: "panic", "{info}");
        previous(info);
    }));
    log::info!(
        "{name} {} started (pid {}) with {:?}; logs in {}",
        crate::VERSION,
        std::process::id(),
        std::env::args().skip(1).collect::<Vec<_>>(),
        dir().display()
    );
    writing.then_some(path)
}

/// Send `log` records at `default` and above (or at the level in
/// `CODEWIKI_LOG`) to standard error only, for a command run in a terminal.
pub fn init_console(default: LevelFilter) {
    install(Logger::new(None, level(default), true));
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
        assert_eq!(app.parent().unwrap(), dir());
        assert_ne!(path("app"), path("backend"));
    }

    #[test]
    fn the_folder_is_the_data_folder_unless_the_environment_names_one() {
        assert_eq!(dir_from(None), data_dir().join("logs"));
        assert_eq!(dir_from(Some(OsString::new())), data_dir().join("logs"));
        // A relative folder is fixed against the working folder, so that a
        // program started from here writes to the same place.
        let relative = dir_from(Some("logs".into()));
        assert!(relative.is_absolute(), "{}", relative.display());
        assert_eq!(relative, std::env::current_dir().unwrap().join("logs"));
        let absolute = std::env::temp_dir().join("codewiki-logs");
        assert_eq!(dir_from(Some(absolute.clone().into())), absolute);
    }

    #[test]
    fn the_level_comes_from_the_environment_or_the_default() {
        assert_eq!(level_from(None, LevelFilter::Info), LevelFilter::Info);
        assert_eq!(level_from(None, LevelFilter::Warn), LevelFilter::Warn);
        assert_eq!(
            level_from(Some("debug"), LevelFilter::Info),
            LevelFilter::Debug
        );
        assert_eq!(
            level_from(Some(" TRACE "), LevelFilter::Info),
            LevelFilter::Trace
        );
        assert_eq!(
            level_from(Some("info"), LevelFilter::Warn),
            LevelFilter::Info
        );
        assert_eq!(level_from(Some("off"), LevelFilter::Info), LevelFilter::Off);
        assert_eq!(
            level_from(Some("loud"), LevelFilter::Warn),
            LevelFilter::Warn
        );
    }

    #[test]
    fn a_failed_move_is_tried_again_only_after_more_growth() {
        assert_eq!(next_rotation(0), LIMIT);
        assert_eq!(next_rotation(LIMIT), LIMIT);
        assert_eq!(next_rotation(LIMIT + 10), 2 * LIMIT + 10);
    }

    fn send(logger: &Logger, level: log::Level, message: &str) {
        logger.log(
            &Record::builder()
                .args(format_args!("{message}"))
                .level(level)
                .target("test")
                .build(),
        );
    }

    #[test]
    fn records_at_the_level_and_above_reach_the_file() {
        let scratch = Scratch::new("logger");
        let log = scratch.0.join("logs").join("backend.log");
        let logger = Logger::new(Some(log.clone()), LevelFilter::Info, false);
        send(&logger, log::Level::Info, "listening");
        send(&logger, log::Level::Debug, "too detailed");
        send(&logger, log::Level::Warn, "slow");
        let text = fs::read_to_string(&log).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "{text}");
        assert!(lines[0].ends_with(" INFO  test: listening"), "{}", lines[0]);
        assert!(lines[1].ends_with(" WARN  test: slow"), "{}", lines[1]);
        // Each line starts with its time.
        assert_eq!(&lines[0][4..5], "-");
        assert_eq!(&lines[0][23..24], "Z");
    }

    #[test]
    fn a_logger_without_a_file_only_prints() {
        let logger = Logger::new(None, LevelFilter::Info, false);
        send(&logger, log::Level::Info, "nothing to write to");
        assert!(logger.sink.lock().unwrap().is_none());
    }

    #[test]
    fn a_full_file_is_moved_aside_while_logging() {
        let scratch = Scratch::new("full");
        fs::create_dir_all(&scratch.0).unwrap();
        let log = scratch.0.join("app.log");
        let logger = Logger::new(Some(log.clone()), LevelFilter::Info, false);
        send(&logger, log::Level::Info, &"x".repeat(LIMIT as usize + 1));
        send(&logger, log::Level::Info, "fresh");
        assert!(fs::read_to_string(&log).unwrap().ends_with("test: fresh\n"));
        assert!(scratch.0.join("app.old.log").exists());
    }

    /// Another process holds the file and Windows refuses the move: the
    /// logger goes on appending and does not try again on the next line.
    #[cfg(windows)]
    #[test]
    fn a_file_held_elsewhere_is_kept_and_appended_to() {
        use std::os::windows::fs::OpenOptionsExt as _;
        const FILE_SHARE_READ: u32 = 1;
        const FILE_SHARE_WRITE: u32 = 2;

        let scratch = Scratch::new("held");
        fs::create_dir_all(&scratch.0).unwrap();
        let log = scratch.0.join("mcp.log");
        fs::write(&log, vec![b'x'; LIMIT as usize + 1]).unwrap();
        // Open without FILE_SHARE_DELETE, so the file cannot be renamed.
        let _held = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .open(&log)
            .unwrap();
        let logger = Logger::new(Some(log.clone()), LevelFilter::Info, false);
        send(&logger, log::Level::Info, "still here");
        assert!(!scratch.0.join("mcp.old.log").exists());
        assert!(
            fs::read_to_string(&log)
                .unwrap()
                .ends_with("test: still here\n")
        );
        let sink = logger.sink.lock().unwrap();
        let sink = sink.as_ref().unwrap();
        assert!(sink.rotate_at > sink.len + LIMIT / 2);
    }
}
