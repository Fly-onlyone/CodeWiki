//! Client for the CodeWiki backend.
//!
//! Clients hold no index of their own. They find a running backend, starting
//! a local one when none is running, and send every request to it.

mod remote;

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde_json::Value;

pub use remote::Remote;

/// Explicit backend address, for a backend running in Docker or on another
/// machine. Requires `CODEWIKI_TOKEN`.
const URL_ENV: &str = "CODEWIKI_URL";
const TOKEN_ENV: &str = "CODEWIKI_TOKEN";
const BACKEND_FILE: &str = "backend.json";
/// How long a backend started here may take to answer. It answers in under a
/// second on a warm machine; a cold disk or a virus scan of a new program can
/// take much longer.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// How a client came to the backend it uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Named by `CODEWIKI_URL`.
    Env,
    /// The backend saved in the app's settings (`remote.json`).
    Saved,
    /// The backend on this machine, found running or started.
    Local,
}

#[derive(Debug, Clone)]
pub struct Backend {
    url: String,
    token: String,
    origin: Origin,
    agent: ureq::Agent,
    /// What this client says it is in [`cw_core::CLIENT_HEADER`], if anything.
    client: Option<&'static str>,
}

/// What the MCP bridge names itself with [`Backend::with_client`].
pub use cw_core::MCP_CLIENT;

/// How long `GET /v1/status` may take. It is asked every few seconds and
/// answers within about a second (its count of the proposals is bounded), so
/// a backend that takes longer is not answering.
const STATUS_TIMEOUT: Duration = Duration::from_secs(3);

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        // Error responses carry a JSON message worth showing to the agent.
        .http_status_as_error(false)
        .build()
        .into()
}

fn process_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// The address `CODEWIKI_URL` names, when it is set. It wins over a saved
/// backend.
pub fn env_remote() -> Option<String> {
    process_env(URL_ENV)
}

/// Whether clients use a backend in Docker or on another machine, named by
/// `CODEWIKI_URL` or saved in the app's settings, rather than one on this
/// machine. Such a backend is not this user's to stop.
pub fn uses_remote() -> bool {
    env_remote().is_some() || saved_remote().is_some_and(|remote| remote.in_use)
}

/// The backend saved in the app's settings, without its token.
pub fn saved_remote() -> Option<Remote> {
    remote::saved_in(&cw_core::paths::data_dir())
}

/// Check that a CodeWiki backend answers at `url` and accepts `token`, and
/// return its version. `token: None` checks with the saved token. Nothing is
/// saved.
pub fn check_remote(url: &str, token: Option<&str>) -> Result<String, String> {
    check_remote_in(&cw_core::paths::data_dir(), url, token)
}

fn check_remote_in(data_dir: &Path, url: &str, token: Option<&str>) -> Result<String, String> {
    let url = remote::normalized(url)?;
    let token = match token.map(str::trim).filter(|token| !token.is_empty()) {
        Some(token) => token.to_string(),
        None => remote::token_in(data_dir)?,
    };
    let version =
        health_version(&url).ok_or_else(|| format!("no CodeWiki backend answers at {url}"))?;
    // Health needs no token; a request that does shows a wrong one now, not
    // after it is saved.
    Backend::new(url.clone(), token, Origin::Saved)
        .get("/v1/repos")
        .map_err(|e| format!("the backend at {url} did not accept the token: {e}"))?;
    Ok(version)
}

/// Save a backend's address and token and make every client on this machine
/// use it: the app and an agent's MCP bridge alike. `token: None` keeps the
/// token saved for the same address. Check it with [`check_remote`] first.
pub fn save_remote(url: &str, token: Option<&str>) -> Result<(), String> {
    remote::save_in(&cw_core::paths::data_dir(), url, token)
}

/// Go back to the backend on this machine. The saved address and token are
/// kept for the next time.
pub fn use_local() -> Result<(), String> {
    remote::set_in_use_in(&cw_core::paths::data_dir(), false)
}

/// Forget the saved backend and its token.
pub fn forget_remote() -> Result<(), String> {
    remote::forget_in(&cw_core::paths::data_dir())
}

/// Percent-encode a query-string value.
pub fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn read_backend_file(data_dir: &Path) -> Option<(String, String)> {
    let text = std::fs::read_to_string(data_dir.join(BACKEND_FILE)).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    Some((
        value["url"].as_str()?.to_string(),
        value["token"].as_str()?.to_string(),
    ))
}

/// The version a backend reports, when it answers `/v1/health`. A backend
/// that answers without a version reports an empty string.
fn health_version(url: &str) -> Option<String> {
    let mut response = agent(Duration::from_secs(2))
        .get(&format!("{url}/v1/health"))
        .call()
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body: Value = response.body_mut().read_json().unwrap_or_default();
    Some(body["version"].as_str().unwrap_or_default().to_string())
}

/// Whether a backend reporting `theirs` is older than this program. One that
/// cannot be compared counts as not older, so it is kept.
fn is_older(theirs: &str, ours: &str) -> bool {
    match (semver::Version::parse(theirs), semver::Version::parse(ours)) {
        (Ok(theirs), Ok(ours)) => theirs < ours,
        _ => false,
    }
}

/// How long an old backend may take to stop after it is asked to: it first
/// finishes the requests it is running.
const STOP_TIMEOUT: Duration = Duration::from_secs(30);

/// A backend this process started.
#[derive(Debug)]
struct Started {
    child: Child,
    /// Whether `backend.json` has named its process: it listened once.
    published: bool,
}

impl Started {
    fn new(child: Child) -> Self {
        Self {
            child,
            published: false,
        }
    }
}

/// The backends this process starts.
#[derive(Debug, Default)]
struct Starts {
    /// The backend this process started last. Until `backend.json` names its
    /// process it may still be starting, however long that takes (a cold
    /// disk, a virus scan of a new program), and a second start would only
    /// race it: `codewiki serve` takes no lock of its own, and the two would
    /// write `backend.json` in turn. The app may try twice at once, by itself
    /// and by the user's "Try again". One that listened once and does not
    /// answer now is ending or hung, and a new one may start beside it: a
    /// backend that ends deletes `backend.json` only while the file is its
    /// own.
    last: Option<Started>,
    /// A call is starting a backend outside the lock; the others wait for it.
    spawning: bool,
    /// No backend is started while this is set ([`hold_starts`]).
    held: bool,
}

static STARTS: Mutex<Starts> = Mutex::new(Starts {
    last: None,
    spawning: false,
    held: false,
});

/// The process that `backend.json` in `data_dir` names, if it names one.
fn published_pid(data_dir: &Path) -> Option<u32> {
    let text = std::fs::read_to_string(data_dir.join(BACKEND_FILE)).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    value["pid"]
        .as_u64()
        .and_then(|pid| u32::try_from(pid).ok())
}

fn starts() -> MutexGuard<'static, Starts> {
    lock(&STARTS)
}

fn lock(starts: &Mutex<Starts>) -> MutexGuard<'_, Starts> {
    starts.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Keep this process from starting a backend, or let it again. The app holds
/// starts while an update replaces the programs: a backend started then
/// would run the program being replaced.
pub fn hold_starts(held: bool) {
    starts().held = held;
}

impl Starts {
    /// Note when `backend.json` in `data_dir` names the backend started last.
    fn notice_published(&mut self, data_dir: &Path) {
        if let Some(last) = &mut self.last
            && !last.published
            && published_pid(data_dir) == Some(last.child.id())
        {
            last.published = true;
        }
    }

    /// Whether a backend this process started may still be starting: one is
    /// being started, or the one started last runs and has never been named
    /// in `backend.json`.
    fn starting(&mut self, data_dir: &Path) -> bool {
        if self.spawning {
            return true;
        }
        self.notice_published(data_dir);
        self.last
            .as_mut()
            .is_some_and(|last| !last.published && matches!(last.child.try_wait(), Ok(None)))
    }

    /// The process of the backend started last and how it ended, once it
    /// has. `None` while a start is under way.
    fn ended(&mut self) -> Option<(u32, std::process::ExitStatus)> {
        if self.spawning {
            return None;
        }
        let last = self.last.as_mut()?;
        let status = last.child.try_wait().ok().flatten()?;
        Some((last.child.id(), status))
    }

    /// Take a start on: delete `backend.json` in `data_dir` and mark the
    /// start under way, for [`spawn_claimed`] to make. Refused while starts
    /// are held.
    fn claim(&mut self, data_dir: &Path) -> Result<(), String> {
        if self.held {
            return Err("no backend is started while CodeWiki is being updated".into());
        }
        // The file names no backend that answers, or one that is ending: it
        // would only mislead the wait for the new one. A backend that ends
        // later leaves the new one's file alone.
        let _ = std::fs::remove_file(data_dir.join(BACKEND_FILE));
        self.spawning = true;
        Ok(())
    }
}

/// Make the start that [`Starts::claim`] took on, and keep its child.
fn spawn_claimed(
    starts: &Mutex<Starts>,
    spawn: &mut impl FnMut() -> Result<Child, String>,
) -> Result<(), String> {
    // Outside the lock: starting a program can take a while (a virus scan of
    // it), and the app calls `hold_starts` on its UI thread. `spawning` keeps
    // the other calls waiting meanwhile.
    let child = spawn();
    let mut starts = lock(starts);
    starts.spawning = false;
    starts.last = Some(Started::new(child?));
    Ok(())
}

/// Start `codewiki serve` in the background.
fn spawn_backend() -> Result<Child, String> {
    let binary = codewiki_binary();
    log::info!("starting a backend: {}", binary.display());
    let mut serve = Command::new(&binary);
    serve
        .arg("serve")
        // The backend writes `backend.log` itself, in the folder this
        // program logs to, a relative `CODEWIKI_LOG_DIR` included.
        .env(cw_core::logs::LOG_DIR_ENV, cw_core::logs::dir())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        // Not inherited: a pipe from here that nobody reads any more (an
        // IDE that stopped this program, an agent host) fills up and
        // stalls the backend. Its own log has what it reports.
        .stderr(Stdio::null());
    // `codewiki` is a console program. Started by the desktop app, which
    // has no console, it would get a console window of its own: a black
    // terminal on top of the app, and on Windows 11 the backend then waits
    // for that terminal before it serves (147 s on a slow machine, against
    // 0.6 s without one, in a clean-Windows test on 2026-11-13). Closing
    // the terminal would also stop the backend.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        serve.creation_flags(CREATE_NO_WINDOW);
    }
    serve
        .spawn()
        .map_err(|e| format!("could not start the backend ({}): {e}", binary.display()))
}

/// The `codewiki` binary: beside this executable when installed together,
/// otherwise whatever `PATH` provides.
fn codewiki_binary() -> PathBuf {
    let name = if cfg!(windows) {
        "codewiki.exe"
    } else {
        "codewiki"
    };
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(name)))
        .filter(|candidate| candidate.exists())
        .unwrap_or_else(|| PathBuf::from(name))
}

impl Backend {
    /// The backend that was named: by `CODEWIKI_URL`, or else by the app's
    /// settings. `None` when neither names one, and the backend on this
    /// machine is the one to use.
    ///
    /// A named backend that cannot be used is an error, never a reason to
    /// fall back to this machine: the user chose where their wikis are, and a
    /// local backend started in its place would show other ones, or none.
    fn named(
        data_dir: &Path,
        env: impl Fn(&str) -> Option<String>,
    ) -> Option<Result<Self, String>> {
        if let Some(url) = env(URL_ENV) {
            return Some(
                env(TOKEN_ENV)
                    .ok_or_else(|| format!("{URL_ENV} is set but {TOKEN_ENV} is not"))
                    .map(|token| {
                        Self::new(url.trim_end_matches('/').to_string(), token, Origin::Env)
                    }),
            );
        }
        let saved = remote::saved_in(data_dir).filter(|remote| remote.in_use)?;
        let url = saved.url;
        Some(
            remote::token_in(data_dir)
                .map_err(|e| format!("the backend at {url} cannot be used: {e}"))
                .and_then(|token| {
                    if health_version(&url).is_none() {
                        return Err(format!(
                            "the backend at {url}, chosen in Settings, does not answer"
                        ));
                    }
                    Ok(Self::new(url.clone(), token, Origin::Saved))
                }),
        )
    }

    /// Find a backend that runs, and never start one: the one named by
    /// `CODEWIKI_URL` or in the app's settings, or else the one on this
    /// machine that `backend.json` names, when it answers `/v1/health`.
    /// `Ok(None)` when nothing is named and no backend runs here.
    pub fn find() -> Result<Option<Self>, String> {
        Self::find_in(&cw_core::paths::data_dir(), process_env)
    }

    fn find_in(
        data_dir: &Path,
        env: impl Fn(&str) -> Option<String>,
    ) -> Result<Option<Self>, String> {
        if let Some(named) = Self::named(data_dir, env) {
            return named.map(Some);
        }
        Ok(Self::running_in(data_dir).map(|(backend, _)| backend))
    }

    /// The backend that `backend.json` in `data_dir` names, with the version
    /// it reports, when it answers.
    fn running_in(data_dir: &Path) -> Option<(Self, String)> {
        let (url, token) = read_backend_file(data_dir)?;
        let version = health_version(&url)?;
        Some((Self::new(url, token, Origin::Local), version))
    }

    /// Find a backend, starting a local one if needed.
    pub fn connect() -> Result<Self, String> {
        let data_dir = cw_core::paths::data_dir();
        if let Some(named) = Self::named(&data_dir, process_env) {
            match &named {
                Ok(backend) => log::info!(
                    "using the backend at {}, named by {}",
                    backend.url,
                    if backend.origin == Origin::Env {
                        URL_ENV
                    } else {
                        "the app's settings"
                    }
                ),
                Err(error) => log::warn!("{error}"),
            }
            return named;
        }
        if let Some((backend, version)) = Self::running_in(&data_dir) {
            // A backend from before an update keeps serving the old code.
            // Replace it, but never one that is newer: an older client must
            // not stop the backend a newer one started.
            if !is_older(&version, cw_core::VERSION) || !backend.stop() {
                // Its log stays where it was started, which may not be the
                // folder this program logs to.
                log::info!("using the running backend {version} at {}", backend.url);
                return Ok(backend);
            }
            log::info!("stopped the older backend {version} at {}", backend.url);
        }
        Self::start(&data_dir)
    }

    /// Start the backend on this machine and wait until it answers.
    fn start(data_dir: &Path) -> Result<Self, String> {
        Self::start_in(data_dir, &STARTS, spawn_backend, STARTUP_TIMEOUT)
    }

    /// Start a backend with `spawn` and wait up to `timeout` for the
    /// `backend.json` in `data_dir` to name one that answers. While the one
    /// started last may still be starting, that one is waited for instead.
    fn start_in(
        data_dir: &Path,
        starts: &Mutex<Starts>,
        mut spawn: impl FnMut() -> Result<Child, String>,
        timeout: Duration,
    ) -> Result<Self, String> {
        let live = || Self::running_in(data_dir).map(|(backend, _)| backend);
        let claimed = {
            let mut starts = lock(starts);
            if starts.starting(data_dir) {
                false
            } else {
                starts.claim(data_dir)?;
                true
            }
        };
        if claimed {
            spawn_claimed(starts, &mut spawn)?;
        } else {
            log::info!("waiting for the backend this program started");
        }
        let mut spawned = claimed;
        let log = cw_core::logs::path("backend");
        let mut waited = Instant::now();
        while waited.elapsed() < timeout {
            // The health checks below run without the lock: each may take
            // seconds, and the app calls `hold_starts` on its UI thread.
            if let Some(backend) = live() {
                lock(starts).notice_published(data_dir);
                log::info!(
                    "the backend at {} answered after {} ms",
                    backend.url,
                    waited.elapsed().as_millis()
                );
                return Ok(backend);
            }
            // A backend that has ended will never answer; say so now rather
            // than at the end of the wait.
            let ended = lock(starts).ended();
            let Some((pid, status)) = ended else {
                std::thread::sleep(Duration::from_millis(150));
                continue;
            };
            // Another client may have started one at the same moment.
            if let Some(backend) = live() {
                return Ok(backend);
            }
            if spawned {
                return Err(format!(
                    "the backend stopped before it answered ({status}); see {}",
                    log.display()
                ));
            }
            // The one waited for was ending, not starting: a backend asked to
            // stop finishes its requests first. Start one now, once, unless
            // another call has since: it is still the same one, ended.
            let claimed = {
                let mut starts = lock(starts);
                if starts.ended().is_some_and(|(now, _)| now == pid) {
                    starts.claim(data_dir)?;
                    true
                } else {
                    false
                }
            };
            if claimed {
                log::info!("the backend this program started ended ({status}); starting another");
                spawn_claimed(starts, &mut spawn)?;
                spawned = true;
                waited = Instant::now();
            }
        }
        Err(format!(
            "the backend did not start within {} seconds; see {}",
            timeout.as_secs(),
            log.display()
        ))
    }

    /// Base URL of the backend this client talks to.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// How this client came to its backend.
    pub fn origin(&self) -> Origin {
        self.origin
    }

    /// Whether the backend was named, by `CODEWIKI_URL` or in the app's
    /// settings, rather than found or started on this machine. A named
    /// backend is not this client's to stop.
    pub fn is_remote(&self) -> bool {
        self.origin != Origin::Local
    }

    /// Ask the backend to stop once its running requests are done.
    pub fn shutdown(&self) -> Result<(), String> {
        Self::finish(
            self.authorized(self.agent.post(&format!("{}/v1/shutdown", self.url)))
                .send_empty(),
        )
        .map(|_| ())
    }

    /// Whether the backend still answers `/v1/health`.
    pub fn answers(&self) -> bool {
        health_version(&self.url).is_some()
    }

    /// Stop the backend and wait until it no longer answers. Returns false
    /// when it could not be asked or is still running after [`STOP_TIMEOUT`].
    pub fn stop(&self) -> bool {
        if self.shutdown().is_err() {
            return false;
        }
        let asked = Instant::now();
        while asked.elapsed() < STOP_TIMEOUT {
            if !self.answers() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(150));
        }
        false
    }

    fn new(url: String, token: String, origin: Origin) -> Self {
        Self {
            url,
            token,
            origin,
            // Generation on a large repository can take minutes.
            agent: agent(Duration::from_secs(600)),
            client: None,
        }
    }

    /// The same backend, with every request saying what this client is. The
    /// MCP bridge passes [`MCP_CLIENT`], so the backend can tell the app when
    /// an agent last called. The app itself sends nothing.
    pub fn with_client(mut self, client: &'static str) -> Self {
        self.client = Some(client);
        self
    }

    /// `request` with the token, and with what this client is when it says.
    fn authorized<B>(&self, request: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B> {
        let request = request.header("Authorization", &format!("Bearer {}", self.token));
        match self.client {
            Some(client) => request.header(cw_core::CLIENT_HEADER, client),
            None => request,
        }
    }

    /// What the backend is doing (`GET /v1/status`): the wiki updates that
    /// are running and when a bridge last called. `None` from a backend too
    /// old to have the route.
    pub fn status(&self) -> Result<Option<Value>, String> {
        let mut response = self
            .authorized(agent(STATUS_TIMEOUT).get(&format!("{}/v1/status", self.url)))
            .call()
            .map_err(|e| format!("backend request failed: {e}"))?;
        let status = response.status();
        if status == ureq::http::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(format!("backend returned {status}"));
        }
        response
            .body_mut()
            .read_json()
            .map(Some)
            .map_err(|e| format!("reading the backend response failed: {e}"))
    }

    fn finish(
        response: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    ) -> Result<Value, String> {
        let (status, value) = Self::read(response)?;
        Self::judge(status, value)
    }

    /// The status and the body of an answer; a body that is not JSON is its
    /// text.
    fn read(
        response: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    ) -> Result<(ureq::http::StatusCode, Value), String> {
        let mut response = response.map_err(|e| format!("backend request failed: {e}"))?;
        let status = response.status();
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|e| format!("reading the backend response failed: {e}"))?;
        Ok((
            status,
            serde_json::from_str(&text).unwrap_or(Value::String(text)),
        ))
    }

    /// The body of an answer that succeeded, else the backend's error.
    fn judge(status: ureq::http::StatusCode, value: Value) -> Result<Value, String> {
        if status.is_success() {
            Ok(value)
        } else {
            Err(value["error"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| format!("backend returned {status}")))
        }
    }

    pub fn get(&self, path: &str) -> Result<Value, String> {
        Self::finish(
            self.authorized(self.agent.get(&format!("{}{path}", self.url)))
                .call(),
        )
    }

    pub fn put(&self, path: &str, body: &Value) -> Result<Value, String> {
        Self::finish(
            self.authorized(self.agent.put(&format!("{}{path}", self.url)))
                .send_json(body),
        )
    }

    pub fn post(&self, path: &str, body: &Value) -> Result<Value, String> {
        Self::finish(
            self.authorized(self.agent.post(&format!("{}{path}", self.url)))
                .send_json(body),
        )
    }

    pub fn delete(&self, path: &str) -> Result<Value, String> {
        Self::finish(
            self.authorized(self.agent.delete(&format!("{}{path}", self.url)))
                .call(),
        )
    }

    /// Id of the repository at `path`, registering it on first use.
    pub fn repo_id(&self, path: &str) -> Result<String, String> {
        let added = self.post("/v1/repos", &serde_json::json!({ "path": path }));
        match added {
            Ok(repo) => repo["id"]
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "backend returned a repository without an id".to_string()),
            // A remote backend does not see this machine's paths; match the
            // repositories it knows by id or folder name instead.
            Err(add_error) => {
                let wanted = path.replace('\\', "/");
                let name = wanted
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or("");
                let repos = self.get("/v1/repos")?;
                repos
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|r| {
                        r["id"] == path
                            || r["name"]
                                .as_str()
                                .is_some_and(|n| n.eq_ignore_ascii_case(name))
                    })
                    .and_then(|r| r["id"].as_str().map(str::to_string))
                    .ok_or(add_error)
            }
        }
    }

    /// Approve (`approve` true) or reject several proposals of `repo` in one
    /// call. One result per id, in the order given: a proposal that cannot be
    /// decided does not stop the others. An older backend has no such route;
    /// the error says so, and the proposals can still be decided one by one.
    pub fn decide_proposals(
        &self,
        repo: &str,
        approve: bool,
        ids: &[String],
    ) -> Result<Vec<DecideResult>, String> {
        let body = serde_json::json!({
            "decision": if approve { "approve" } else { "reject" },
            "ids": ids,
        });
        let url = format!("{}/v1/repos/{repo}/proposals/decide", self.url);
        let (status, answer) = Self::read(self.authorized(self.agent.post(&url)).send_json(&body))?;
        // An older backend's router answers 404, or 405 when the route of one
        // proposal takes the path, and gives no error of its own.
        let unknown = matches!(
            status,
            ureq::http::StatusCode::NOT_FOUND | ureq::http::StatusCode::METHOD_NOT_ALLOWED
        );
        if unknown && answer["error"].is_null() {
            return Err(
                "this backend cannot decide several proposals at once; update it".to_string(),
            );
        }
        parse_decide_results(&Self::judge(status, answer)?)
    }
}

/// What the backend did with one proposal of a batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecideResult {
    pub id: String,
    pub ok: bool,
    /// Vault-relative path of the note, when the decision went through.
    pub note_path: Option<String>,
    /// Why the decision did not go through.
    pub error: Option<String>,
}

fn parse_decide_results(answer: &Value) -> Result<Vec<DecideResult>, String> {
    let results = answer["results"]
        .as_array()
        .ok_or("the backend answered without results")?;
    let text = |item: &Value, key: &str| item[key].as_str().map(str::to_string);
    Ok(results
        .iter()
        .map(|item| DecideResult {
            id: text(item, "id").unwrap_or_default(),
            ok: item["ok"].as_bool().unwrap_or(false),
            note_path: text(item, "note_path"),
            error: text(item, "error"),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_query_values() {
        assert_eq!(encode("src/math.rs"), "src%2Fmath.rs");
        assert_eq!(encode("a b&c=d"), "a%20b%26c%3Dd");
        assert_eq!(encode("Ünï"), "%C3%9Cn%C3%AF");
        assert_eq!(encode("safe-_.~09azAZ"), "safe-_.~09azAZ");
    }

    #[test]
    fn batch_results_keep_their_order_and_reasons() {
        let answer = serde_json::json!({ "results": [
            { "id": "a", "ok": true, "note_path": "01/A.md" },
            { "id": "b", "ok": false, "error": "proposal b is not pending" },
        ] });
        let results = parse_decide_results(&answer).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].ok && results[0].note_path.as_deref() == Some("01/A.md"));
        assert!(!results[1].ok && results[1].error.is_some());
        assert!(parse_decide_results(&serde_json::json!({})).is_err());
    }

    /// A backend that answers every request with `status` and `body`.
    fn answering_backend(status: &'static str, body: &'static str) -> String {
        use std::io::{BufRead, BufReader, Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut reader = BufReader::new(stream.unwrap());
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line.trim_end().is_empty() {
                        break;
                    }
                    if let Some((name, value)) = line.trim_end().split_once(": ")
                        && name.eq_ignore_ascii_case("content-length")
                    {
                        length = value.parse().unwrap();
                    }
                }
                // Closing on a body nobody read resets the connection.
                reader.read_exact(&mut vec![0; length]).unwrap();
                let mut stream = reader.into_inner();
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
                stream.flush().unwrap();
            }
        });
        url
    }

    #[test]
    fn a_backend_without_the_decide_route_says_so_by_its_status() {
        let ids = ["a".to_string(), "b".to_string()];
        // An older backend's router: no such route, or the route of one
        // proposal taking the path and refusing the method.
        for status in ["404 Not Found", "405 Method Not Allowed"] {
            let backend = Backend::new(answering_backend(status, ""), "s".into(), Origin::Local);
            assert_eq!(
                backend.decide_proposals("r", true, &ids).unwrap_err(),
                "this backend cannot decide several proposals at once; update it",
                "{status}"
            );
        }
        // A backend that has the route keeps its own reason.
        let url = answering_backend("404 Not Found", r#"{"error":"no repository 404"}"#);
        let backend = Backend::new(url, "s".into(), Origin::Local);
        assert_eq!(
            backend.decide_proposals("r", true, &ids).unwrap_err(),
            "no repository 404"
        );
    }

    #[test]
    fn only_an_older_backend_is_replaced() {
        assert!(is_older("0.1.0", "0.2.0"));
        assert!(is_older("0.2.0-rc.1", "0.2.0"));
        assert!(!is_older("0.2.0", "0.2.0"));
        // A newer backend was started by a newer client: keep it.
        assert!(!is_older("0.3.0", "0.2.0"));
        // A backend that does not say its version is kept.
        assert!(!is_older("", "0.2.0"));
    }

    /// A backend that answers `/v1/health` until it is asked to stop.
    fn stoppable_backend(token: &'static str) -> String {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let stream = stream.unwrap();
                let mut reader = BufReader::new(stream);
                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();
                let mut authorized = false;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line.trim_end().is_empty() {
                        break;
                    }
                    authorized |= line.trim_end() == format!("Authorization: Bearer {token}")
                        || line.trim_end() == format!("authorization: Bearer {token}");
                }
                let shutdown = request_line.starts_with("POST /v1/shutdown ");
                let stop = shutdown && authorized;
                let (status, body) = if stop {
                    ("202 Accepted", r#"{"status":"stopping"}"#)
                } else if shutdown {
                    (
                        "401 Unauthorized",
                        r#"{"error":"missing or invalid bearer token"}"#,
                    )
                } else {
                    ("200 OK", r#"{"status":"ok","version":"0.0.1"}"#)
                };
                let mut stream = reader.into_inner();
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
                stream.flush().unwrap();
                if stop {
                    // Dropping the listener refuses every later connection.
                    return;
                }
            }
        });
        url
    }

    /// A backend that answers each request with the headers it was sent, as
    /// a JSON object with lower-case names. `/v1/status` is unknown to it, as
    /// to a backend from before that route.
    fn echoing_backend() -> String {
        use std::io::{BufRead, BufReader, Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let stream = stream.unwrap();
                let mut reader = BufReader::new(stream);
                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();
                let mut headers = serde_json::Map::new();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    let Some((name, value)) = line.trim_end().split_once(": ") else {
                        break;
                    };
                    headers.insert(name.to_ascii_lowercase(), value.into());
                }
                // Closing on a body nobody read resets the connection, and
                // the client then never sees the answer.
                let length = headers
                    .get("content-length")
                    .and_then(Value::as_str)
                    .and_then(|length| length.parse().ok())
                    .unwrap_or(0);
                reader.read_exact(&mut vec![0; length]).unwrap();
                let (status, body) = if request_line.starts_with("GET /v1/status ") {
                    ("404 Not Found", String::new())
                } else {
                    ("200 OK", Value::Object(headers).to_string())
                };
                let mut stream = reader.into_inner();
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
                stream.flush().unwrap();
            }
        });
        url
    }

    #[test]
    fn only_a_client_that_names_itself_sends_the_client_header() {
        let url = echoing_backend();
        let app = Backend::new(url.clone(), "secret".into(), Origin::Local);
        let sent = app.get("/v1/repos").unwrap();
        assert_eq!(sent["authorization"], "Bearer secret");
        assert_eq!(sent[cw_core::CLIENT_HEADER], Value::Null);

        let bridge = Backend::new(url, "secret".into(), Origin::Local).with_client(MCP_CLIENT);
        for sent in [
            bridge.get("/v1/repos").unwrap(),
            bridge.post("/v1/repos", &serde_json::json!({})).unwrap(),
            bridge
                .put("/v1/repos/a/note", &serde_json::json!({}))
                .unwrap(),
            bridge.delete("/v1/repos/a").unwrap(),
        ] {
            assert_eq!(sent["authorization"], "Bearer secret");
            assert_eq!(sent[cw_core::CLIENT_HEADER], "mcp");
        }
    }

    #[test]
    fn a_backend_without_the_status_route_has_no_status() {
        let backend = Backend::new(echoing_backend(), "secret".into(), Origin::Local);
        assert_eq!(backend.status(), Ok(None));
        // A backend that is gone is an error, not "no status".
        let gone = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", gone.local_addr().unwrap());
        drop(gone);
        assert!(
            Backend::new(url, "secret".into(), Origin::Local)
                .status()
                .is_err()
        );
    }

    #[test]
    fn stop_asks_with_the_token_and_waits_until_the_backend_is_gone() {
        let url = stoppable_backend("secret");
        assert_eq!(health_version(&url).as_deref(), Some("0.0.1"));
        let wrong = Backend::new(url.clone(), "wrong".into(), Origin::Local);
        assert!(!wrong.stop());
        assert!(wrong.answers());
        let backend = Backend::new(url, "secret".into(), Origin::Local);
        assert!(backend.stop());
        assert!(!backend.answers());
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "codewiki-client-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn no_env(_: &str) -> Option<String> {
        None
    }

    /// An address nothing listens on.
    fn dead_address() -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", listener.local_addr().unwrap())
    }

    #[test]
    fn the_environment_names_the_backend_first() {
        let dir = temp_dir("env");
        // Nothing names a backend: the one on this machine is used.
        assert!(Backend::named(&dir, no_env).is_none());

        let env = |name: &str| match name {
            "CODEWIKI_URL" => Some("http://docker:7411/".to_string()),
            "CODEWIKI_TOKEN" => Some("from-env".to_string()),
            _ => None,
        };
        let backend = Backend::named(&dir, env).unwrap().unwrap();
        assert_eq!(backend.url(), "http://docker:7411");
        assert_eq!(backend.origin(), Origin::Env);
        assert!(backend.is_remote());

        // An address without its token is an error, not a fall back.
        let env = |name: &str| (name == "CODEWIKI_URL").then(|| "http://docker:7411".to_string());
        let error = Backend::named(&dir, env).unwrap().unwrap_err();
        assert!(error.contains("CODEWIKI_TOKEN"), "{error}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn a_saved_backend_is_used_and_never_replaced_by_a_local_one() {
        // Made up: not a real token.
        const TOKEN: &str = "sample-token-for-tests-0042";
        let dir = temp_dir("saved");
        let url = stoppable_backend(TOKEN);
        remote::save_in(&dir, &url, Some(TOKEN)).unwrap();

        let backend = Backend::named(&dir, no_env).unwrap().unwrap();
        assert_eq!(backend.url(), url);
        assert_eq!(backend.origin(), Origin::Saved);
        assert!(backend.is_remote());

        // The environment still wins over it.
        let env = |name: &str| match name {
            "CODEWIKI_URL" => Some("http://docker:7411".to_string()),
            "CODEWIKI_TOKEN" => Some("from-env".to_string()),
            _ => None,
        };
        assert_eq!(
            Backend::named(&dir, env).unwrap().unwrap().origin(),
            Origin::Env
        );

        // It stops answering: that is an error. `connect` returns it and
        // starts nothing on this machine.
        assert!(backend.stop());
        let error = Backend::named(&dir, no_env).unwrap().unwrap_err();
        assert!(
            error.contains(&url) && error.contains("does not answer"),
            "{error}"
        );

        // Only the user's choice brings the local backend back.
        remote::set_in_use_in(&dir, false).unwrap();
        assert!(Backend::named(&dir, no_env).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_check_finds_a_dead_address_and_a_wrong_token() {
        let dir = temp_dir("check");
        let error = check_remote_in(&dir, "nas:7411", Some("t")).unwrap_err();
        assert!(error.contains("http://"), "{error}");
        // No token given and none saved.
        assert!(check_remote_in(&dir, "http://nas:7411", None).is_err());

        let dead = dead_address();
        let error = check_remote_in(&dir, &dead, Some("t")).unwrap_err();
        assert!(error.contains("no CodeWiki backend answers"), "{error}");

        let url = guarded_backend("right");
        let error = check_remote_in(&dir, &url, Some("wrong")).unwrap_err();
        assert!(error.contains("did not accept the token"), "{error}");
        assert_eq!(
            check_remote_in(&dir, &format!("{url}/"), Some("right")).as_deref(),
            Ok("0.0.1")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_uses_a_running_backend_and_starts_none() {
        let dir = temp_dir("find");
        // Nothing is named and nothing runs: not found, and nothing started.
        assert!(matches!(Backend::find_in(&dir, no_env), Ok(None)));
        assert!(starts().last.is_none());
        assert!(std::fs::read_dir(&dir).unwrap().next().is_none());

        // `backend.json` names a backend that answers: that one, with its
        // token.
        let url = guarded_backend("right");
        let file = serde_json::json!({ "url": url, "token": "right" });
        std::fs::write(dir.join(BACKEND_FILE), file.to_string()).unwrap();
        let found = Backend::find_in(&dir, no_env).unwrap().unwrap();
        assert_eq!(found.url(), url);
        assert_eq!(found.origin(), Origin::Local);
        assert_eq!(found.get("/v1/repos"), Ok(serde_json::json!([])));

        // A named backend wins, as in `connect`.
        let env = |name: &str| match name {
            "CODEWIKI_URL" => Some("http://docker:7411".to_string()),
            "CODEWIKI_TOKEN" => Some("from-env".to_string()),
            _ => None,
        };
        let named = Backend::find_in(&dir, env).unwrap().unwrap();
        assert_eq!(named.origin(), Origin::Env);

        // The file names a backend that is gone: not found, and the file is
        // left for whoever starts the next one.
        let file = serde_json::json!({ "url": dead_address(), "token": "t" });
        std::fs::write(dir.join(BACKEND_FILE), file.to_string()).unwrap();
        assert!(matches!(Backend::find_in(&dir, no_env), Ok(None)));
        assert!(dir.join(BACKEND_FILE).exists());
        assert!(starts().last.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A program that runs for about `seconds` and does nothing else: the
    /// stand-in for a backend's process.
    #[cfg(windows)]
    fn stand_in(seconds: u32) -> Child {
        Command::new("ping")
            .args(["-n", &(seconds + 1).to_string(), "127.0.0.1"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    }

    /// The starts of a process whose last backend is `child`, not yet named
    /// in `backend.json`.
    #[cfg(windows)]
    fn started_last(child: Child) -> Mutex<Starts> {
        Mutex::new(Starts {
            last: Some(Started::new(child)),
            ..Starts::default()
        })
    }

    /// `backend.json` in `dir`, naming `url` and, when given, the process
    /// that listens there.
    #[cfg(windows)]
    fn publish(dir: &Path, url: &str, pid: Option<u32>) {
        let file = serde_json::json!({ "url": url, "token": "t", "pid": pid });
        std::fs::write(dir.join(BACKEND_FILE), file.to_string()).unwrap();
    }

    #[cfg(windows)]
    fn end(starts: Mutex<Starts>) {
        let starts = starts.into_inner().unwrap_or_else(PoisonError::into_inner);
        if let Some(mut last) = starts.last {
            let _ = last.child.kill();
            let _ = last.child.wait();
        }
    }

    #[cfg(windows)]
    #[test]
    fn a_start_waits_for_the_backend_this_program_is_starting() {
        let dir = temp_dir("starting");
        let child = stand_in(30);
        let pid = child.id();
        let starts = started_last(child);
        // It writes its file a moment later, once it listens.
        let url = guarded_backend("t");
        let writer = {
            let (dir, url) = (dir.clone(), url.clone());
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(300));
                publish(&dir, &url, Some(pid));
            })
        };
        let mut spawns = 0;
        let spawn = || {
            spawns += 1;
            Err("a second backend was started".to_string())
        };
        let backend = Backend::start_in(&dir, &starts, spawn, Duration::from_secs(10)).unwrap();
        writer.join().unwrap();
        assert_eq!(backend.url(), url);
        assert_eq!(spawns, 0);
        // It listened: from now on, a silence of it means it is ending.
        assert!(lock(&starts).last.as_ref().unwrap().published);
        end(starts);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn a_backend_still_starting_holds_up_a_start_however_long_it_takes() {
        let dir = temp_dir("slow");
        // A cold disk or a virus scan keeps it from listening past the first
        // wait, and the user presses "Try again": no second one. The guard
        // keeps no age, so how long it has run does not matter.
        let starts = started_last(stand_in(30));
        let mut spawns = 0;
        for _ in 0..2 {
            let spawn = || {
                spawns += 1;
                Err("a second backend was started".to_string())
            };
            let error =
                Backend::start_in(&dir, &starts, spawn, Duration::from_secs(1)).unwrap_err();
            assert!(error.contains("did not start within"), "{error}");
        }
        assert_eq!(spawns, 0);
        end(starts);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn a_backend_that_ends_unanswered_while_waited_for_is_replaced_once() {
        let dir = temp_dir("ending");
        let starts = started_last(stand_in(30));
        // A stale file of another backend, which does not answer.
        publish(&dir, &dead_address(), None);
        let url = guarded_backend("t");
        let ended = std::sync::atomic::AtomicBool::new(false);
        let mut spawns = 0;
        std::thread::scope(|scope| {
            // It ends without answering while the start waits for it.
            scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(500));
                let mut starts = lock(&starts);
                let last = starts.last.as_mut().unwrap();
                last.child.kill().unwrap();
                last.child.wait().unwrap();
                ended.store(true, std::sync::atomic::Ordering::SeqCst);
            });
            let spawn = || {
                spawns += 1;
                // Only once the one waited for ended: the start did wait.
                assert!(ended.load(std::sync::atomic::Ordering::SeqCst));
                assert!(!dir.join(BACKEND_FILE).exists(), "the stale file stayed");
                publish(&dir, &url, None);
                Ok(stand_in(30))
            };
            let backend = Backend::start_in(&dir, &starts, spawn, Duration::from_secs(10)).unwrap();
            assert_eq!(backend.url(), url);
        });
        assert_eq!(spawns, 1);
        end(starts);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn two_starts_waiting_for_a_backend_that_ends_replace_it_once() {
        let dir = temp_dir("two");
        let starts = started_last(stand_in(30));
        let url = guarded_backend("t");
        let spawns = std::sync::atomic::AtomicUsize::new(0);
        std::thread::scope(|scope| {
            let start = || {
                let spawn = || {
                    spawns.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    // A slow start, as of a program being scanned.
                    std::thread::sleep(Duration::from_millis(300));
                    publish(&dir, &url, None);
                    Ok(stand_in(30))
                };
                Backend::start_in(&dir, &starts, spawn, Duration::from_secs(10))
            };
            let first = scope.spawn(start);
            let second = scope.spawn(start);
            std::thread::sleep(Duration::from_millis(500));
            {
                let mut starts = lock(&starts);
                let last = starts.last.as_mut().unwrap();
                last.child.kill().unwrap();
                last.child.wait().unwrap();
            }
            assert_eq!(first.join().unwrap().unwrap().url(), url);
            assert_eq!(second.join().unwrap().unwrap().url(), url);
        });
        assert_eq!(spawns.load(std::sync::atomic::Ordering::SeqCst), 1);
        end(starts);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn a_backend_that_listened_and_is_silent_now_holds_up_no_start() {
        let dir = temp_dir("published");
        // It listened and published, and now drains after a shutdown: its
        // file is still there, its port no longer answers.
        let child = stand_in(30);
        publish(&dir, &dead_address(), Some(child.id()));
        let starts = started_last(child);
        let url = guarded_backend("t");
        let mut spawns = 0;
        let spawn = || {
            spawns += 1;
            publish(&dir, &url, None);
            Ok(stand_in(30))
        };
        let asked = Instant::now();
        let backend = Backend::start_in(&dir, &starts, spawn, Duration::from_secs(10)).unwrap();
        assert!(asked.elapsed() < Duration::from_secs(3), "it waited");
        assert_eq!(backend.url(), url);
        assert_eq!(spawns, 1);
        end(starts);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_backend_is_started_while_starts_are_held() {
        let dir = temp_dir("held");
        let starts = Mutex::new(Starts {
            held: true,
            ..Starts::default()
        });
        let mut spawns = 0;
        let spawn = || {
            spawns += 1;
            Err("started".to_string())
        };
        let error = Backend::start_in(&dir, &starts, spawn, Duration::from_secs(2)).unwrap_err();
        assert!(error.contains("being updated"), "{error}");
        assert_eq!(spawns, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A backend that answers `/v1/health` to anyone and everything else
    /// only to the holder of `token`.
    fn guarded_backend(token: &'static str) -> String {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { return };
                let mut reader = BufReader::new(stream);
                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();
                let mut authorized = false;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    let line = line.trim_end();
                    if line.is_empty() {
                        break;
                    }
                    authorized |=
                        line.eq_ignore_ascii_case(&format!("authorization: Bearer {token}"));
                }
                let (status, body) = if request_line.starts_with("GET /v1/health ") {
                    ("200 OK", r#"{"status":"ok","version":"0.0.1"}"#)
                } else if authorized {
                    ("200 OK", "[]")
                } else {
                    (
                        "401 Unauthorized",
                        r#"{"error":"missing or invalid bearer token"}"#,
                    )
                };
                let mut stream = reader.into_inner();
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
                stream.flush().unwrap();
            }
        });
        url
    }
}
