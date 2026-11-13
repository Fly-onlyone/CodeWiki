//! The app's log: `%LOCALAPPDATA%\CodeWiki\logs\app.log`.
//!
//! A release build has no console, so nothing the app or GPUI reports was
//! kept anywhere. Every `log` record at Info and above goes to this file,
//! GPUI's own included (it names the graphics adapter it picked), and so does
//! every panic. `CODEWIKI_LOG=debug` or `trace` keeps more.

use std::fs::File;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

use cw_core::logs;
use log::{LevelFilter, Log, Metadata, Record};

/// The open file and how long it is, so that it can be moved aside once full.
struct Sink {
    file: File,
    len: u64,
}

struct FileLogger {
    path: PathBuf,
    level: LevelFilter,
    /// `None` after a failed reopen; records are then dropped.
    sink: Mutex<Option<Sink>>,
}

impl FileLogger {
    fn open(path: &std::path::Path) -> Option<Sink> {
        let file = logs::open(path).ok()?;
        let len = file.metadata().map(|meta| meta.len()).unwrap_or(0);
        Some(Sink { file, len })
    }
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!(
            "{} {:<5} {}: {}\n",
            logs::timestamp(SystemTime::now()),
            record.level(),
            record.target(),
            record.args()
        );
        // A panic while the lock was held must not silence the log.
        let mut sink = self
            .sink
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if sink.as_ref().is_some_and(|s| s.len > logs::LIMIT) {
            // Close the file first: Windows will not move an open one.
            *sink = None;
            *sink = Self::open(&self.path);
        }
        if let Some(sink) = sink.as_mut()
            && sink.file.write_all(line.as_bytes()).is_ok()
        {
            sink.len += line.len() as u64;
        }
    }

    fn flush(&self) {}
}

/// The level asked for in `CODEWIKI_LOG`, Info when unset or unknown.
fn level_from(value: Option<&str>) -> LevelFilter {
    match value.map(str::to_ascii_lowercase).as_deref() {
        Some("trace") => LevelFilter::Trace,
        Some("debug") => LevelFilter::Debug,
        Some("warn") => LevelFilter::Warn,
        Some("error") => LevelFilter::Error,
        _ => LevelFilter::Info,
    }
}

/// Send `log` records and panics to the app's log file. Call first in `main`.
/// Without a writable data folder the app runs on without a log.
pub fn init() {
    let path = logs::path("app");
    let Some(sink) = FileLogger::open(&path) else {
        return;
    };
    let level = level_from(std::env::var("CODEWIKI_LOG").ok().as_deref());
    let logger = FileLogger {
        path,
        level,
        sink: Mutex::new(Some(sink)),
    };
    if log::set_boxed_logger(Box::new(logger)).is_err() {
        return;
    }
    log::set_max_level(level);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!(target: "panic", "{info}");
        previous(info);
    }));
    log::info!(
        "CodeWiki {} started with {:?}",
        cw_core::VERSION,
        std::env::args().skip(1).collect::<Vec<_>>()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_level_comes_from_the_environment_and_defaults_to_info() {
        assert_eq!(level_from(None), LevelFilter::Info);
        assert_eq!(level_from(Some("debug")), LevelFilter::Debug);
        assert_eq!(level_from(Some("TRACE")), LevelFilter::Trace);
        assert_eq!(level_from(Some("warn")), LevelFilter::Warn);
        assert_eq!(level_from(Some("loud")), LevelFilter::Info);
    }
}
