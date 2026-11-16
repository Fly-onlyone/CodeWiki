//! One running app for each installed program.
//!
//! The app stays in the tray when its window closes, so starting it again
//! must not start a second one: the launch hands what it was asked to open
//! to the app that is running, which shows its window, and ends.
//!
//! "The same app" is the same program file. An installed copy and a build
//! from the sources are two apps, each with its window; otherwise a build
//! under test would hand over to the installed version. `CODEWIKI_INSTANCE=new`
//! starts a separate app whatever is running, for tests and for checks that
//! must not bring another window to the front.
//!
//! Two files in the settings folder, both named after the program file:
//!
//! - `instance-<key>.lock` is held locked by the running app for as long as
//!   it lives. Windows drops the lock when the process ends, however it ends.
//! - `instance-<key>.txt` says where that app listens: `port=`, `token=` and
//!   `pid=`, one per line. It is a second file because a locked file cannot
//!   be read by another process.
//!
//! A launch that finds the lock taken sends one line of JSON to
//! `127.0.0.1:<port>` and waits for `ok`. The token keeps a program that
//! merely found the port from driving the app; it guards against nothing
//! that can read the user's settings folder.
//!
//! The app never refuses to start. A launch that can neither take the lock
//! nor reach the running app (it is just starting, or just ending) tries
//! again for a few seconds and then runs by itself.

use std::fs::{File, TryLockError};
use std::hash::{BuildHasher, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::prefs;
use crate::startup::{self, Launch};

/// Set to `new` to start a separate app whatever is running.
const INSTANCE_ENV: &str = "CODEWIKI_INSTANCE";
/// Version of the line a launch sends. A running app of another version
/// answers `no` to a line it does not know, and the launch runs by itself.
const PROTOCOL: u64 = 1;
/// How long a launch keeps trying to take the lock or reach the running app.
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(10);
/// The pause between two tries.
const RETRY_PAUSE: Duration = Duration::from_millis(150);
/// How long connecting to the running app may take; it is on this machine.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
/// How long the running app may take to answer a request.
const REPLY_TIMEOUT: Duration = Duration::from_secs(3);
/// How long the running app waits for the line of a launch that connected.
const READ_TIMEOUT: Duration = Duration::from_secs(2);
/// The longest line the running app reads: a path, a view and a token.
const LINE_LIMIT: u64 = 64 * 1024;

/// What a second launch asks the running app to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The folder to open, as a full path.
    pub repo: Option<String>,
    /// The note or view to open on.
    pub view: Option<String>,
    /// The launch asked for the tray (`--tray`): do not show the window.
    pub tray: bool,
}

impl Request {
    /// What `launch`, started in the working folder `cwd`, asks for.
    pub fn new(launch: &Launch, cwd: Option<&Path>) -> Self {
        Self {
            repo: startup::named_repo(launch.repo.as_deref(), cwd),
            view: launch.view.clone(),
            tray: launch.tray,
        }
    }
}

/// How this launch stands to the app that may be running already.
pub enum Acquired {
    /// This is the app. It holds the lock until [`Instance::release`] or the
    /// end of the process.
    Primary(Instance),
    /// The running app took the request; this process has nothing left to do.
    HandedOver,
    /// Run without the lock: asked for, or the lock could not be had.
    Separate,
}

/// The lock of the running app and the listener launches reach it through.
pub struct Instance {
    /// Dropping it gives the lock up.
    _lock: File,
    info: PathBuf,
    port: u16,
    closing: Arc<AtomicBool>,
}

impl Instance {
    /// Stop being the running app: a launch from now on becomes the app
    /// itself. Called before the app ends, and before an update starts the
    /// new version, which would otherwise hand over to this one as it ends.
    pub fn release(self) {
        self.closing.store(true, Ordering::Relaxed);
        let _ = std::fs::remove_file(&self.info);
        // Wake the listener, so that it sees the flag and lets go of the port.
        let _ = TcpStream::connect_timeout(&local(self.port), CONNECT_TIMEOUT);
    }
}

/// The instance this process holds, kept so that [`release`] finds it.
static HELD: Mutex<Option<Instance>> = Mutex::new(None);

/// Become the running app, or hand `launch` to the one that is. `deliver`
/// receives, on another thread, each request a later launch hands over; it
/// returns whether the app took it.
pub fn acquire(launch: &Launch, deliver: impl Fn(Request) -> bool + Send + 'static) -> Acquired {
    if std::env::var(INSTANCE_ENV).is_ok_and(|value| value == "new") {
        log::info!("{INSTANCE_ENV}=new: a separate app, whatever is running");
        return Acquired::Separate;
    }
    let (Some(dir), Ok(exe)) = (prefs::dir(), std::env::current_exe()) else {
        log::warn!("no settings folder or program path: running as a separate app");
        return Acquired::Separate;
    };
    let cwd = std::env::current_dir().ok();
    let request = Request::new(launch, cwd.as_deref());
    acquire_in(&dir, &key(&exe), &request, ACQUIRE_TIMEOUT, deliver)
}

/// Keep `instance` for [`release`].
pub fn hold(instance: Instance) {
    *HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(instance);
}

/// Give up being the running app, if this process is it.
pub fn release() {
    let held = HELD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();
    if let Some(instance) = held {
        instance.release();
        log::info!("no longer the running app");
    }
}

/// [`acquire`] with everything it reads from the system passed in.
fn acquire_in(
    dir: &Path,
    key: &str,
    request: &Request,
    timeout: Duration,
    deliver: impl Fn(Request) -> bool + Send + 'static,
) -> Acquired {
    if let Err(e) = std::fs::create_dir_all(dir) {
        log::warn!("no settings folder ({e}): running as a separate app");
        return Acquired::Separate;
    }
    let lock_path = dir.join(format!("instance-{key}.lock"));
    let info_path = dir.join(format!("instance-{key}.txt"));
    let started = Instant::now();
    loop {
        let lock = match File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
        {
            Ok(lock) => lock,
            Err(e) => {
                log::warn!("{}: {e}; running as a separate app", lock_path.display());
                return Acquired::Separate;
            }
        };
        match lock.try_lock() {
            Ok(()) => {
                return match listen(lock, info_path, deliver) {
                    Ok(instance) => {
                        log::info!(
                            "this is the running app; launches reach it on port {}",
                            instance.port
                        );
                        Acquired::Primary(instance)
                    }
                    Err(e) => {
                        log::warn!(
                            "could not listen for later launches ({e}): running as a separate app"
                        );
                        Acquired::Separate
                    }
                };
            }
            Err(TryLockError::WouldBlock) => {
                if hand_over(&info_path, request) {
                    return Acquired::HandedOver;
                }
            }
            Err(TryLockError::Error(e)) => {
                log::warn!("{}: {e}; running as a separate app", lock_path.display());
                return Acquired::Separate;
            }
        }
        // The app that holds the lock did not answer: it is starting and has
        // not said where it listens, or it is ending and still holds the
        // lock. Either way the next try settles it.
        if started.elapsed() >= timeout {
            log::warn!("the running app does not answer: running as a separate app");
            return Acquired::Separate;
        }
        std::thread::sleep(RETRY_PAUSE);
    }
}

/// Start listening for later launches and say where in the info file.
fn listen(
    lock: File,
    info: PathBuf,
    deliver: impl Fn(Request) -> bool + Send + 'static,
) -> std::io::Result<Instance> {
    let listener = TcpListener::bind(local(0))?;
    let port = listener.local_addr()?.port();
    let token = new_token();
    // Written whole, then moved into place: a launch never reads half of it.
    let draft = info.with_extension("tmp");
    std::fs::write(&draft, info_text(port, &token, std::process::id()))?;
    std::fs::rename(&draft, &info)?;
    let closing = Arc::new(AtomicBool::new(false));
    let stop = closing.clone();
    std::thread::Builder::new()
        .name("instance".into())
        .spawn(move || {
            for stream in listener.incoming() {
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                let Ok(stream) = stream else {
                    continue;
                };
                let taken = read_request(&stream, &token).is_some_and(&deliver);
                let mut stream = stream;
                let _ = stream.write_all(if taken { b"ok\n" } else { b"no\n" });
            }
        })?;
    Ok(Instance {
        _lock: lock,
        info,
        port,
        closing,
    })
}

/// The request a launch that connected sends, when it knows the token.
fn read_request(stream: &TcpStream, token: &str) -> Option<Request> {
    stream.set_read_timeout(Some(READ_TIMEOUT)).ok()?;
    let mut line = String::new();
    BufReader::new(stream.take(LINE_LIMIT))
        .read_line(&mut line)
        .ok()?;
    let request = decode(&line, token);
    if request.is_none() {
        log::warn!("a launch sent a request this app does not take");
    }
    request
}

/// Send `request` to the running app the info file names. True when it
/// answered that it took it.
fn hand_over(info: &Path, request: &Request) -> bool {
    let Some(target) = std::fs::read_to_string(info)
        .ok()
        .and_then(|text| parse_info(&text))
    else {
        return false;
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&local(target.port), CONNECT_TIMEOUT) else {
        return false;
    };
    // This process is the one the user has just started, so it may hand its
    // right to the foreground to the app that will show the window.
    #[cfg(windows)]
    crate::win32::allow_foreground(target.pid);
    let line = encode(request, &target.token);
    if stream.set_read_timeout(Some(REPLY_TIMEOUT)).is_err()
        || stream.write_all(line.as_bytes()).is_err()
    {
        return false;
    }
    let mut reply = String::new();
    let _ = BufReader::new(stream).read_line(&mut reply);
    let taken = reply.trim_end() == "ok";
    if taken {
        log::info!("handed over to the running app (process {})", target.pid);
    }
    taken
}

fn local(port: u16) -> SocketAddr {
    SocketAddr::from((Ipv4Addr::LOCALHOST, port))
}

/// Where a running app listens, from its info file.
#[derive(Debug, PartialEq, Eq)]
struct Target {
    port: u16,
    token: String,
    pid: u32,
}

fn info_text(port: u16, token: &str, pid: u32) -> String {
    format!("port={port}\ntoken={token}\npid={pid}\n")
}

fn parse_info(text: &str) -> Option<Target> {
    let field = |name: &str| {
        text.lines()
            .find_map(|line| line.trim().strip_prefix(name)?.strip_prefix('='))
    };
    Some(Target {
        port: field("port")?.parse().ok()?,
        token: field("token")
            .filter(|token| !token.is_empty())?
            .to_string(),
        pid: field("pid")?.parse().ok()?,
    })
}

/// The name the two files carry for the program at `exe`: the same for the
/// same file however its path is spelled in upper and lower case, as Windows
/// reads paths. FNV-1a, because the name must be the same in every process.
fn key(exe: &Path) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in exe.to_string_lossy().to_lowercase().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// 128 bits nobody can guess, from the randomly seeded hashers the standard
/// library keeps for its maps.
fn new_token() -> String {
    let word = || {
        std::collections::hash_map::RandomState::new()
            .build_hasher()
            .finish()
    };
    format!("{:016x}{:016x}", word(), word())
}

/// The line a launch sends.
fn encode(request: &Request, token: &str) -> String {
    let mut line = json!({
        "v": PROTOCOL,
        "token": token,
        "repo": request.repo,
        "view": request.view,
        "tray": request.tray,
    })
    .to_string();
    line.push('\n');
    line
}

/// The request in `line`, when it is of this version and carries `token`.
fn decode(line: &str, token: &str) -> Option<Request> {
    let value: Value = serde_json::from_str(line.trim_end()).ok()?;
    if value["v"].as_u64()? != PROTOCOL || value["token"].as_str()? != token {
        return None;
    }
    let text = |name: &str| {
        value[name]
            .as_str()
            .filter(|text| !text.is_empty())
            .map(str::to_string)
    };
    Some(Request {
        repo: text("repo"),
        view: text("view"),
        tray: value["tray"].as_bool().unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    /// A fresh folder under the system's temporary folder, removed when the
    /// test ends.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(test: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("codewiki-instance-{test}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn request(repo: Option<&str>, view: Option<&str>) -> Request {
        Request {
            repo: repo.map(str::to_string),
            view: view.map(str::to_string),
            tray: false,
        }
    }

    #[test]
    fn the_key_is_the_same_for_the_same_program_file() {
        let installed = key(Path::new(
            r"C:\Users\Me\AppData\Local\Programs\CodeWiki\codewiki-app.exe",
        ));
        assert_eq!(installed.len(), 16);
        assert_eq!(
            installed,
            key(Path::new(
                r"c:\users\me\appdata\local\programs\codewiki\CODEWIKI-APP.EXE"
            ))
        );
        // A build from the sources is another app.
        assert_ne!(
            installed,
            key(Path::new(r"D:\code\CodeWiki\target\debug\codewiki-app.exe"))
        );
    }

    #[test]
    fn a_request_survives_the_line_it_is_sent_in() {
        let sent = Request {
            repo: Some(r"D:\code\my app".into()),
            view: Some("edit:Notes \"quoted\"".into()),
            tray: true,
        };
        let line = encode(&sent, "secret");
        assert!(line.ends_with('\n'));
        assert_eq!(line.lines().count(), 1);
        assert_eq!(decode(&line, "secret"), Some(sent));
        assert_eq!(
            decode(&encode(&request(None, None), "secret"), "secret"),
            Some(request(None, None))
        );
    }

    #[test]
    fn a_line_without_the_token_or_of_another_version_is_refused() {
        let line = encode(&request(Some(r"D:\code\app"), None), "secret");
        assert_eq!(decode(&line, "other"), None);
        assert_eq!(decode(&line.replace("\"v\":1", "\"v\":2"), "secret"), None);
        assert_eq!(decode("", "secret"), None);
        assert_eq!(decode("not json", "secret"), None);
        assert_eq!(decode(r#"{"v":1}"#, "secret"), None);
    }

    #[test]
    fn the_info_file_names_port_token_and_process() {
        let text = info_text(49152, "abc", 4242);
        assert_eq!(
            parse_info(&text),
            Some(Target {
                port: 49152,
                token: "abc".into(),
                pid: 4242,
            })
        );
        // Windows line ends, and lines in another order.
        assert_eq!(
            parse_info("pid=7\r\ntoken=t\r\nport=80\r\n"),
            Some(Target {
                port: 80,
                token: "t".into(),
                pid: 7,
            })
        );
        // Half a file, or a broken one, names nothing.
        assert_eq!(parse_info("port=80\n"), None);
        assert_eq!(parse_info("port=x\ntoken=t\npid=7\n"), None);
        assert_eq!(parse_info("port=80\ntoken=\npid=7\n"), None);
        assert_eq!(parse_info(""), None);
    }

    #[test]
    fn tokens_differ() {
        let (one, two) = (new_token(), new_token());
        assert_eq!(one.len(), 32);
        assert_ne!(one, two);
    }

    #[test]
    fn a_second_launch_hands_over_and_a_release_frees_the_place() {
        let scratch = Scratch::new("handover");
        let (seen, requests) = mpsc::channel();
        let first = acquire_in(
            &scratch.0,
            "k",
            &request(None, None),
            Duration::from_secs(5),
            move |request| seen.send(request).is_ok(),
        );
        let Acquired::Primary(first) = first else {
            panic!("the first launch must become the app");
        };
        assert!(scratch.0.join("instance-k.lock").is_file());
        assert!(scratch.0.join("instance-k.txt").is_file());

        // The second launch reaches the first, which gets its request.
        let asked = request(Some(r"D:\code\app"), Some("graph:"));
        let second = acquire_in(&scratch.0, "k", &asked, Duration::from_secs(5), |_| {
            panic!("the second launch is not the app")
        });
        assert!(matches!(second, Acquired::HandedOver));
        assert_eq!(
            requests.recv_timeout(Duration::from_secs(5)).unwrap(),
            asked
        );

        // The program at another path is another app.
        let other = acquire_in(
            &scratch.0,
            "other",
            &request(None, None),
            Duration::from_secs(5),
            |_| true,
        );
        assert!(matches!(other, Acquired::Primary(_)));

        // Once the first lets go, the next launch is the app.
        first.release();
        assert!(!scratch.0.join("instance-k.txt").exists());
        let third = acquire_in(
            &scratch.0,
            "k",
            &request(None, None),
            Duration::from_secs(5),
            |_| true,
        );
        assert!(matches!(third, Acquired::Primary(_)));
    }

    #[test]
    fn a_launch_runs_by_itself_when_the_running_app_refuses_or_is_silent() {
        // An app that takes no request: every launch gets `no`.
        let scratch = Scratch::new("refused");
        let first = acquire_in(
            &scratch.0,
            "k",
            &request(None, None),
            Duration::from_secs(5),
            |_| false,
        );
        assert!(matches!(first, Acquired::Primary(_)));
        let second = acquire_in(
            &scratch.0,
            "k",
            &request(None, None),
            Duration::from_millis(300),
            |_| true,
        );
        assert!(matches!(second, Acquired::Separate));

        // A lock with no info file beside it: an app that is just starting,
        // or one that ended without a word. The launch does not wait forever.
        let silent = Scratch::new("silent");
        std::fs::create_dir_all(&silent.0).unwrap();
        let held = File::create(silent.0.join("instance-k.lock")).unwrap();
        held.try_lock().unwrap();
        let launch = acquire_in(
            &silent.0,
            "k",
            &request(None, None),
            Duration::from_millis(300),
            |_| true,
        );
        assert!(matches!(launch, Acquired::Separate));
    }
}
