//! Error reports to Sentry from the programs that run for a long time: the
//! app, the backend and the MCP bridge.
//!
//! A program reports only when one of these holds:
//! - `SENTRY_DSN` is set in its environment, as on a developer's machine;
//! - the program was built with a DSN (`CODEWIKI_SENTRY_DSN` at compile time,
//!   which the release workflow sets) and the user turned on "Send error
//!   reports" in Settings → Diagnostics, which writes [`CONSENT_FILE`] in
//!   `%APPDATA%\CodeWiki`.
//!
//! Each program reads that file itself, so the MCP bridge and a backend an
//! agent started follow the switch too. Turning it on takes effect at each
//! program's next start. Turning it off holds at once, because every report
//! is checked against the file again first.
//!
//! [`init`] replaces `cw_core::logs::init`: records still go to the log file,
//! and an Error record also becomes a report, while Info and Warn records go
//! along with the next report as its trail. A panic is reported with its
//! backtrace. Before anything leaves the machine it is scrubbed ([`scrub`]):
//! no host name, no absolute path, and no query string, headers or body of a
//! request.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use log::LevelFilter;
use sentry::integrations::log::{LogFilter, SentryLogger};

pub mod scrub;

/// The SDK, for the backend's request traces.
pub use sentry;

/// Environment variable naming the project to report to. It needs no
/// switch: whoever sets it chose to report.
pub const DSN_ENV: &str = "SENTRY_DSN";

/// Environment variable naming the environment of the reports. Without it,
/// `development` for a debug build and `production` for a release build.
pub const ENVIRONMENT_ENV: &str = "SENTRY_ENVIRONMENT";

/// Environment variable with the share of backend requests traced, from 0
/// to 1. [`DEFAULT_TRACES_RATE`] without it.
pub const TRACES_RATE_ENV: &str = "SENTRY_TRACES_SAMPLE_RATE";

/// The share of backend requests traced when [`TRACES_RATE_ENV`] is unset.
pub const DEFAULT_TRACES_RATE: f32 = 0.1;

/// The file in `%APPDATA%\CodeWiki` that holds the user's choice: `on`, or
/// absent.
pub const CONSENT_FILE: &str = "error_reports.txt";

/// The DSN the build carries, set by the release workflow.
const BUILT_IN_DSN: Option<&str> = option_env!("CODEWIKI_SENTRY_DSN");

/// How long the switch, once read, is trusted before it is read again.
const RECHECK: Duration = Duration::from_secs(5);

/// Whether and how this program can report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reporting {
    /// `SENTRY_DSN` names a project: reports go there, whatever the switch
    /// says.
    Environment,
    /// The build carries a DSN, and the switch decides.
    Switch,
    /// Neither: this build never reports.
    Unavailable,
}

/// How this program can report, for the Settings page.
pub fn reporting() -> Reporting {
    match (env_dsn(), built_in_dsn()) {
        (Some(_), _) => Reporting::Environment,
        (None, Some(_)) => Reporting::Switch,
        (None, None) => Reporting::Unavailable,
    }
}

fn env_dsn() -> Option<String> {
    std::env::var(DSN_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn built_in_dsn() -> Option<&'static str> {
    // A workflow that has no DSN to give still sets the variable, empty.
    BUILT_IN_DSN.filter(|value| !value.trim().is_empty())
}

/// The DSN to report to, if any, and what governs it: `env` (the value of
/// `SENTRY_DSN`) always wins; the one the build carries is used only while
/// the user's switch is on.
fn choose(
    env: Option<String>,
    built_in: Option<&str>,
    consent: bool,
) -> Option<(String, Reporting)> {
    if let Some(dsn) = env {
        return Some((dsn, Reporting::Environment));
    }
    match built_in {
        Some(dsn) if consent => Some((dsn.to_string(), Reporting::Switch)),
        _ => None,
    }
}

/// Where the user's choice is kept: `%APPDATA%\CodeWiki\error_reports.txt`.
pub fn consent_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .filter(|root| !root.is_empty())
        .map(|root| PathBuf::from(root).join("CodeWiki").join(CONSENT_FILE))
}

/// Whether the user turned on "Send error reports".
pub fn consent() -> bool {
    consent_path().is_some_and(|path| read_consent(&path))
}

fn read_consent(path: &Path) -> bool {
    fs::read_to_string(path).is_ok_and(|text| text.trim() == "on")
}

/// Keep the user's choice. Off removes the file.
pub fn set_consent(on: bool) -> io::Result<()> {
    let path = consent_path().ok_or_else(|| io::Error::other("APPDATA is not set"))?;
    write_consent(&path, on)
}

fn write_consent(path: &Path, on: bool) -> io::Result<()> {
    if on {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(path, "on")
    } else {
        match fs::remove_file(path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    }
}

/// What governs the reports of this process, once [`init`] started them.
static MODE: OnceLock<Reporting> = OnceLock::new();

/// Whether a report may leave now: always under `SENTRY_DSN`, and under the
/// switch while it is still on.
fn allowed() -> bool {
    match MODE.get() {
        Some(Reporting::Environment) => true,
        Some(Reporting::Switch) => switch_on(),
        Some(Reporting::Unavailable) | None => false,
    }
}

/// The switch, read again at most every [`RECHECK`]: a backend traces
/// requests, and need not read a file for each.
fn switch_on() -> bool {
    static LAST: Mutex<Option<(Instant, bool)>> = Mutex::new(None);
    let mut last = LAST.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((at, on)) = *last
        && at.elapsed() < RECHECK
    {
        return on;
    }
    let on = consent();
    *last = Some((Instant::now(), on));
    on
}

/// Whether this process sends reports now: [`init`] started them and the
/// user has not turned them off since. The backend traces a request only
/// then.
pub fn sending() -> bool {
    sentry::Hub::current()
        .client()
        .is_some_and(|client| client.is_enabled())
        && allowed()
}

/// Keeps the reports going; hold it until `main` returns, whose end sends
/// what is still queued.
#[must_use = "reports stop when the guard is dropped"]
pub struct Guard {
    _client: Option<sentry::ClientInitGuard>,
}

/// Start the logs of program `name` as `cw_core::logs::init` does, and its
/// error reports when they are allowed (see the crate's documentation).
/// Call first in `main`.
pub fn init(name: &'static str, default: LevelFilter) -> Guard {
    let guard = start(name);
    let reporting = guard.is_some();
    cw_core::logs::init_with(name, default, |file| {
        if reporting {
            Box::new(SentryLogger::with_dest(file).filter(log_filter))
        } else {
            file
        }
    });
    if let Some(mode) = MODE.get() {
        log::info!("error reports are on ({mode:?})");
    }
    Guard { _client: guard }
}

fn start(name: &'static str) -> Option<sentry::ClientInitGuard> {
    let (dsn, mode) = choose(env_dsn(), built_in_dsn(), consent())?;
    // A DSN that does not parse reports nothing, rather than stopping the
    // program.
    let dsn = dsn.trim().parse().ok()?;
    let mut options = sentry::ClientOptions::new()
        .release(format!("codewiki@{}", cw_core::VERSION))
        .environment(environment())
        .send_default_pii(false)
        // An Error record has no backtrace of its own; this gives it the
        // one of the thread that logged it.
        .attach_stacktrace(true)
        .traces_sample_rate(traces_rate(std::env::var(TRACES_RATE_ENV).ok()))
        .before_send(move |event| allowed().then(|| scrub::event(event, name)))
        .before_breadcrumb(|breadcrumb| Some(scrub::breadcrumb(breadcrumb)));
    options.dsn = Some(dsn);
    let guard = sentry::init(options);
    let _ = MODE.set(mode);
    Some(guard)
}

fn environment() -> String {
    std::env::var(ENVIRONMENT_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            if cfg!(debug_assertions) {
                "development"
            } else {
                "production"
            }
            .to_string()
        })
}

/// The share of requests traced, from the value of [`TRACES_RATE_ENV`]. A
/// value that is not a number from 0 to 1 is ignored.
fn traces_rate(value: Option<String>) -> f32 {
    value
        .and_then(|value| value.trim().parse::<f32>().ok())
        .filter(|rate| (0.0..=1.0).contains(rate))
        .unwrap_or(DEFAULT_TRACES_RATE)
}

/// What becomes of a log record: an Error is a report, Info and Warn are
/// the trail of the next one. A panic is reported by Sentry's own hook,
/// with its backtrace, so the record `cw_core::logs` writes of it is left
/// out. So are the records of the crates that send the reports, which could
/// otherwise report about their own sending.
fn log_filter(metadata: &log::Metadata<'_>) -> LogFilter {
    let target = metadata.target();
    if target == "panic"
        || ["sentry", "ureq", "rustls"]
            .iter()
            .any(|own| target.starts_with(own))
    {
        return LogFilter::Ignore;
    }
    match metadata.level() {
        log::Level::Error => LogFilter::Event,
        log::Level::Warn | log::Level::Info => LogFilter::Breadcrumb,
        log::Level::Debug | log::Level::Trace => LogFilter::Ignore,
    }
}

/// Send what is queued, waiting up to two seconds. For a program that is
/// about to end by a path that skips the end of `main`.
pub fn flush() {
    if let Some(client) = sentry::Hub::current().client() {
        client.flush(Some(Duration::from_secs(2)));
    }
}

/// A report that a check can look for: an Error record, which becomes an
/// event when reports are on.
pub fn send_test_report(name: &str) {
    log::error!("test report from {name}, sent on request");
}

/// The hub a task of its own reports through, so that what one request
/// adds to its scope stays out of the others.
pub fn task_hub() -> Arc<sentry::Hub> {
    Arc::new(sentry::Hub::new_from_top(sentry::Hub::current()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh folder under the system's temporary folder, removed when the
    /// test ends.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(test: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("codewiki-telemetry-{test}-{}", std::process::id()));
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
    fn the_environment_wins_and_the_built_in_dsn_needs_the_switch() {
        let env = "https://key@example.invalid/1".to_string();
        let built = "https://key@example.invalid/2";
        assert_eq!(
            choose(Some(env.clone()), Some(built), false),
            Some((env.clone(), Reporting::Environment))
        );
        assert_eq!(
            choose(Some(env.clone()), None, true),
            Some((env, Reporting::Environment))
        );
        assert_eq!(
            choose(None, Some(built), true),
            Some((built.to_string(), Reporting::Switch))
        );
        assert_eq!(choose(None, Some(built), false), None);
        assert_eq!(choose(None, None, true), None);
    }

    #[test]
    fn the_switch_is_a_file_that_says_on() {
        let scratch = Scratch::new("consent");
        let path = scratch.0.join("CodeWiki").join(CONSENT_FILE);
        assert!(!read_consent(&path));
        write_consent(&path, true).unwrap();
        assert!(read_consent(&path));
        assert_eq!(fs::read_to_string(&path).unwrap(), "on");
        write_consent(&path, false).unwrap();
        assert!(!path.exists());
        assert!(!read_consent(&path));
        // Turning off what is off is no error.
        write_consent(&path, false).unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "off").unwrap();
        assert!(!read_consent(&path));
        fs::write(&path, " on\r\n").unwrap();
        assert!(read_consent(&path));
    }

    #[test]
    fn the_trace_rate_is_a_share_or_the_default() {
        assert_eq!(traces_rate(None), DEFAULT_TRACES_RATE);
        assert_eq!(traces_rate(Some("1".into())), 1.0);
        assert_eq!(traces_rate(Some(" 0.25 ".into())), 0.25);
        assert_eq!(traces_rate(Some("0".into())), 0.0);
        assert_eq!(traces_rate(Some("2".into())), DEFAULT_TRACES_RATE);
        assert_eq!(traces_rate(Some("-1".into())), DEFAULT_TRACES_RATE);
        assert_eq!(traces_rate(Some("half".into())), DEFAULT_TRACES_RATE);
    }

    fn filtered(level: log::Level, target: &str) -> LogFilter {
        log_filter(&log::Metadata::builder().level(level).target(target).build())
    }

    #[test]
    fn errors_are_reports_and_info_is_their_trail() {
        assert_eq!(filtered(log::Level::Error, "cw_server"), LogFilter::Event);
        assert_eq!(
            filtered(log::Level::Warn, "cw_server"),
            LogFilter::Breadcrumb
        );
        assert_eq!(
            filtered(log::Level::Info, "codewiki_app"),
            LogFilter::Breadcrumb
        );
        assert_eq!(filtered(log::Level::Debug, "cw_wiki"), LogFilter::Ignore);
        assert_eq!(filtered(log::Level::Trace, "cw_wiki"), LogFilter::Ignore);
    }

    #[test]
    fn panics_and_the_senders_own_records_are_left_out() {
        assert_eq!(filtered(log::Level::Error, "panic"), LogFilter::Ignore);
        assert_eq!(
            filtered(log::Level::Error, "ureq::unversioned"),
            LogFilter::Ignore
        );
        assert_eq!(
            filtered(log::Level::Warn, "rustls::conn"),
            LogFilter::Ignore
        );
        assert_eq!(filtered(log::Level::Error, "sentry"), LogFilter::Ignore);
    }

    #[test]
    fn a_process_that_never_started_reports_sends_nothing() {
        assert!(!sending());
    }
}
