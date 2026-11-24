//! Client for the CodeWiki backend.
//!
//! Clients hold no index of their own. They find a running backend, starting
//! a local one when none is running, and send every request to it.

mod remote;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
/// answers from memory, so a backend that takes longer is not answering.
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
        let live_with_version = || {
            let (url, token) = read_backend_file(&data_dir)?;
            let version = health_version(&url)?;
            Some((Self::new(url, token, Origin::Local), version))
        };
        let live = || live_with_version().map(|(backend, _)| backend);
        if let Some((backend, version)) = live_with_version() {
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
        // No live backend: a stale file would only mislead the wait below.
        let _ = std::fs::remove_file(data_dir.join(BACKEND_FILE));
        let binary = codewiki_binary();
        log::info!("starting a backend: {}", binary.display());
        let mut serve = Command::new(&binary);
        serve
            .arg("serve")
            // The backend writes `backend.json` where this program waits for
            // it. Without the variable, a release `codewiki` found on `PATH`
            // would use the installed CodeWiki's folder, not a development
            // build's (`cw_core::paths::APP_FOLDER`).
            .env(cw_core::paths::DATA_DIR_ENV, &data_dir)
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
        let mut child = serve
            .spawn()
            .map_err(|e| format!("could not start the backend ({}): {e}", binary.display()))?;
        let log = cw_core::logs::path("backend");
        let started = Instant::now();
        while started.elapsed() < STARTUP_TIMEOUT {
            if let Some(backend) = live() {
                log::info!(
                    "the backend at {} answered after {} ms",
                    backend.url,
                    started.elapsed().as_millis()
                );
                return Ok(backend);
            }
            // A backend that has ended will never answer; say so now rather
            // than at the end of the wait.
            if let Ok(Some(status)) = child.try_wait() {
                // Another client may have started one at the same moment.
                return live().ok_or_else(|| {
                    format!(
                        "the backend stopped before it answered ({status}); see {}",
                        log.display()
                    )
                });
            }
            std::thread::sleep(Duration::from_millis(150));
        }
        Err(format!(
            "the backend did not start within {} seconds; see {}",
            STARTUP_TIMEOUT.as_secs(),
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
        let mut response = response.map_err(|e| format!("backend request failed: {e}"))?;
        let status = response.status();
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|e| format!("reading the backend response failed: {e}"))?;
        let value: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
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
