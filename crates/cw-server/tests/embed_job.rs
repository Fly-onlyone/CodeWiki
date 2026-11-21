//! The background embedding job, driven in-process.
//!
//! No test here calls a model server. The job is given a stand-in embedder
//! that embeds with the word-form embedder, the same one held by a test at
//! its first batch, or one whose server is down. The job runs for every
//! embedder, so these tests run on every platform.
//!
//! A test program of its own: `serve` sets the process's embedder and its
//! inline budget, which the routes of `api.rs` must not see.

use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use cw_core::{Error, Result};
use cw_embed::{Embedder, LocalEmbedder};
use cw_server::{Host, ServerConfig, router};
use cw_wiki::semantic::behind;
use cw_wiki::service::{DOCUMENT_KINDS, open_existing_index_in};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const TOKEN: &str = "test-token";
/// How many times a test asks before it gives up: ten seconds at least.
const TRIES: usize = 1000;
const PAUSE: Duration = Duration::from_millis(10);

fn no_env(_: &str) -> Option<String> {
    None
}

/// Have the process embed with the word-form embedder, which empty settings
/// and an empty environment name. Until the embedder slot is configured it
/// holds what the machine's environment names, and an update would call
/// that server. Every harness calls this before its router or `serve` runs.
fn no_model_server() {
    cw_wiki::semantic::configure(&cw_core::settings::Resolved::from_parts(
        &Default::default(),
        no_env,
    ));
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A gate a test shuts to hold a job inside its first batch.
struct Gate {
    shut: Mutex<bool>,
    opened: Condvar,
    /// How many batches came to the gate.
    arrived: AtomicUsize,
}

impl Gate {
    const fn new() -> Self {
        Self {
            shut: Mutex::new(false),
            opened: Condvar::new(),
            arrived: AtomicUsize::new(0),
        }
    }

    /// Shut the gate until the guard is dropped. A test that fails midway
    /// drops it too, so that no job is left waiting.
    fn shut(&'static self) -> Opens {
        *lock(&self.shut) = true;
        Opens(self)
    }

    fn pass(&self) {
        self.arrived.fetch_add(1, Ordering::SeqCst);
        let mut shut = lock(&self.shut);
        while *shut {
            shut = self
                .opened
                .wait(shut)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }

    fn arrived(&self) -> usize {
        self.arrived.load(Ordering::SeqCst)
    }
}

struct Opens(&'static Gate);

impl Drop for Opens {
    fn drop(&mut self) {
        *lock(&self.0.shut) = false;
        self.0.opened.notify_all();
    }
}

/// The embedder a background job is given: another one than the index was
/// made with, as far as the index can tell, and slow enough to be watched.
struct StandIn {
    local: LocalEmbedder,
    /// Each batch passes it first, where a test holds the job.
    gate: Option<&'static Gate>,
}

impl Embedder for StandIn {
    fn name(&self) -> String {
        "stand-in".into()
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if let Some(gate) = self.gate {
            gate.pass();
        }
        std::thread::sleep(Duration::from_millis(60));
        self.local.embed(texts)
    }

    fn batch_size(&self) -> usize {
        1
    }
}

fn stand_in() -> Option<Arc<dyn Embedder>> {
    Some(Arc::new(StandIn {
        local: LocalEmbedder::default(),
        gate: None,
    }))
}

/// The gate of the one test that holds its job.
static HOLD: Gate = Gate::new();

fn held() -> Option<Arc<dyn Embedder>> {
    Some(Arc::new(StandIn {
        local: LocalEmbedder::default(),
        gate: Some(&HOLD),
    }))
}

/// An embedder whose server is down. Each call is counted, in the counter of
/// the test it belongs to.
struct Failing(&'static AtomicUsize);

impl Embedder for Failing {
    fn name(&self) -> String {
        "failing".into()
    }

    fn embed(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(Error::invalid("the server did not answer"))
    }
}

static FAILED: AtomicUsize = AtomicUsize::new(0);

fn failing() -> Option<Arc<dyn Embedder>> {
    Some(Arc::new(Failing(&FAILED)))
}

static FAILED_AT_START: AtomicUsize = AtomicUsize::new(0);

fn failing_at_start() -> Option<Arc<dyn Embedder>> {
    Some(Arc::new(Failing(&FAILED_AT_START)))
}

/// Whether the server of [`down_then_up`] answers yet.
static UP: AtomicBool = AtomicBool::new(false);
static FAILED_WHILE_DOWN: AtomicUsize = AtomicUsize::new(0);

/// An embedder whose server is down until a test brings it up.
fn down_then_up() -> Option<Arc<dyn Embedder>> {
    if UP.load(Ordering::SeqCst) {
        stand_in()
    } else {
        Some(Arc::new(Failing(&FAILED_WHILE_DOWN)))
    }
}

/// Semantic search is off: an update leaves nothing to a job.
fn off() -> Option<Arc<dyn Embedder>> {
    None
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

struct Harness {
    app: Router,
    data: tempfile::TempDir,
    repo: tempfile::TempDir,
}

impl Harness {
    fn new(settings_writable: bool, embedder: fn() -> Option<Arc<dyn Embedder>>) -> Self {
        let data = tempfile::tempdir().unwrap();
        let repo = tempfile::tempdir().unwrap();
        write(
            repo.path(),
            "Cargo.toml",
            "[package]\nname = \"demo\"\ndescription = \"Demo crate\"\n",
        );
        write(
            repo.path(),
            "src/lib.rs",
            "//! Demo crate.\npub mod math;\n\n/// Entry point.\npub fn run() -> u64 { math::double(21) }\n",
        );
        write(
            repo.path(),
            "src/math.rs",
            "//! Arithmetic helpers.\n\n/// Doubles a number.\npub fn double(x: u64) -> u64 { x * 2 }\n\n/// Halves a number.\npub fn half(x: u64) -> u64 { x / 2 }\n",
        );
        no_model_server();
        let app = router(config(data.path(), settings_writable, embedder));
        Self { app, data, repo }
    }

    async fn call(&self, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("authorization", format!("Bearer {TOKEN}"));
        let request = match body {
            Some(json) => request
                .header("content-type", "application/json")
                .body(Body::from(json.to_string()))
                .unwrap(),
            None => request.body(Body::empty()).unwrap(),
        };
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn ok(&self, method: &str, uri: &str, body: Option<Value>) -> Value {
        let (status, body) = self.call(method, uri, body).await;
        assert_eq!(status, StatusCode::OK, "{method} {uri}: {body}");
        body
    }

    /// Start a job and return it as it started.
    async fn start(&self) -> Value {
        let (status, body) = self.call("POST", "/v1/embed/reembed", None).await;
        assert_eq!(
            status,
            StatusCode::ACCEPTED,
            "POST /v1/embed/reembed: {body}"
        );
        body["job"].clone()
    }

    async fn job(&self) -> Value {
        self.ok("GET", "/v1/embed/job", None).await["job"].clone()
    }

    /// The job, once `ready` holds for it.
    async fn job_when(&self, what: &str, ready: impl Fn(&Value) -> bool) -> Value {
        for _ in 0..TRIES {
            let job = self.job().await;
            if ready(&job) {
                return job;
            }
            tokio::time::sleep(PAUSE).await;
        }
        panic!("no job {what}: {}", self.job().await);
    }

    async fn job_in(&self, phase: &str) -> Value {
        self.job_when(&format!("in phase '{phase}'"), |job| job["phase"] == phase)
            .await
    }

    /// Register the repository and build its wiki and index.
    async fn repository(&self) -> String {
        let entry = self
            .ok(
                "POST",
                "/v1/repos",
                Some(json!({ "path": self.repo.path().to_string_lossy() })),
            )
            .await;
        let id = entry["id"].as_str().unwrap().to_string();
        self.ok("POST", &format!("/v1/repos/{id}/update"), None)
            .await;
        id
    }

    /// Whether documents of the repository still wait for a vector of the
    /// embedder `name`.
    fn waits(&self, name: &str) -> bool {
        let index = open_existing_index_in(self.data.path(), self.repo.path())
            .unwrap()
            .expect("the repository has an index");
        behind(&index, name, &DOCUMENT_KINDS).unwrap()
    }
}

fn config(
    data: &Path,
    settings_writable: bool,
    embedder: fn() -> Option<Arc<dyn Embedder>>,
) -> ServerConfig {
    ServerConfig {
        data_dir: data.to_path_buf(),
        token: TOKEN.into(),
        webhook_secret: None,
        settings_writable,
        env: no_env,
        host: Host { embedder },
    }
}

/// Wait until `ready` holds.
async fn until(what: &str, ready: impl Fn() -> bool) {
    for _ in 0..TRIES {
        if ready() {
            return;
        }
        tokio::time::sleep(PAUSE).await;
    }
    panic!("{what} never happened");
}

#[tokio::test]
async fn a_backend_that_listens_beyond_this_machine_answers_no_job_route() {
    let h = Harness::new(false, stand_in);
    for (method, uri) in [
        ("GET", "/v1/embed/job"),
        ("POST", "/v1/embed/reembed"),
        ("POST", "/v1/embed/cancel"),
    ] {
        let (status, body) = h.call(method, uri, None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {uri}");
        assert_eq!(
            body["error"], "this backend's embedding is managed on its own machine",
            "{method} {uri}"
        );
    }
    // The refused reembed started nothing.
    assert_eq!(
        h.ok("GET", "/v1/status", None).await["model_job"],
        Value::Null
    );
}

#[tokio::test]
async fn an_update_leaves_the_rest_to_a_job_that_embeds_every_repository() {
    let h = Harness::new(true, stand_in);
    assert_eq!(h.job().await, Value::Null);
    // The update embeds with the process's embedder, not with the one the
    // job is given, so it leaves every document to a job it starts itself.
    h.repository().await;
    let job = h.job_in("done").await;
    assert_eq!(job["model"], "stand-in");
    assert_eq!(job["repo_count"], 1);
    assert_eq!(job["error"], Value::Null);
    assert_eq!(job["skipped"], json!([]));
    let total = job["docs_total"].as_u64().unwrap();
    assert!(total >= 5, "{job}");
    assert_eq!(job["docs_done"], job["docs_total"]);
    assert!(!h.waits("stand-in"));
    // A job that is over is no longer reported to the tray.
    assert_eq!(
        h.ok("GET", "/v1/status", None).await["model_job"],
        Value::Null
    );
}

#[tokio::test]
async fn one_job_runs_at_a_time_and_cancel_stops_it() {
    let h = Harness::new(true, held);
    let hold = HOLD.shut();
    // The update starts a job by itself, which waits inside its first batch.
    h.repository().await;
    until("a batch at the gate", || HOLD.arrived() >= 1).await;
    let job = h.job_in("embedding").await;
    assert_eq!(job["docs_done"], 0);
    let total = job["docs_total"].as_u64().unwrap();
    assert!(total >= 5, "{job}");
    assert!(job["repo"].is_string(), "{job}");
    // The tray reads this.
    assert_eq!(
        h.ok("GET", "/v1/status", None).await["model_job"],
        json!({ "phase": "embedding", "model": "stand-in", "repo": job["repo"] })
    );
    // One job at a time.
    let (status, body) = h.call("POST", "/v1/embed/reembed", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("another embedding job is running"),
        "{body}"
    );

    // Cancelled inside its first batch: it stores that batch and stops.
    let cancelled = h.ok("POST", "/v1/embed/cancel", None).await;
    assert_eq!(cancelled["job"]["phase"], "embedding");
    drop(hold);
    let job = h.job_in("cancelled").await;
    assert_eq!(job["docs_done"], 1);
    assert!(h.waits("stand-in"));
    assert_eq!(
        h.ok("GET", "/v1/status", None).await["model_job"],
        Value::Null
    );

    // The next job embeds the rest and no more.
    h.start().await;
    let job = h.job_in("done").await;
    assert_eq!(job["docs_total"].as_u64().unwrap(), total - 1);
    assert_eq!(job["docs_done"], job["docs_total"]);
    assert!(!h.waits("stand-in"));

    // Nothing waits now: a job has nothing to do, and says so by being done.
    h.start().await;
    let job = h.job_in("done").await;
    assert_eq!(job["docs_total"], 0);
}

#[tokio::test]
async fn a_job_whose_embedder_fails_says_why_and_can_be_started_again() {
    let h = Harness::new(true, failing);
    h.repository().await;
    let job = h.job_in("failed").await;
    assert_eq!(job["model"], "failing");
    let error = job["error"].as_str().unwrap();
    assert!(error.contains("did not answer"), "{error}");
    // What failed stays to do.
    assert!(h.waits("failing"));

    // A failed job is over: another may start, and tries again.
    let tried = FAILED.load(Ordering::SeqCst);
    let job = h.start().await;
    assert_eq!(job["phase"], "embedding");
    let job = h.job_in("failed").await;
    let error = job["error"].as_str().unwrap();
    assert!(error.contains("did not answer"), "{error}");
    assert!(FAILED.load(Ordering::SeqCst) > tried);
}

#[tokio::test]
async fn a_test_that_succeeds_tries_the_embedding_again() {
    let h = Harness::new(true, down_then_up);
    h.repository().await;
    let job = h.job_in("failed").await;
    let error = job["error"].as_str().unwrap();
    assert!(error.contains("did not answer"), "{error}");
    assert!(FAILED_WHILE_DOWN.load(Ordering::SeqCst) > 0);

    // The server comes up, and someone presses Test.
    UP.store(true, Ordering::SeqCst);
    let result = h.ok("POST", "/v1/settings/test/embed", None).await;
    assert_eq!(result["ok"], true, "{result}");
    let job = h.job_in("done").await;
    assert_eq!(job["model"], "stand-in");
    assert_eq!(job["error"], Value::Null);
    assert!(!h.waits("stand-in"));
}

/// One request to a backend that `serve` runs, over a socket: its status
/// and its body.
async fn request(addr: SocketAddr, method: &'static str, path: &'static str) -> (u16, Value) {
    tokio::task::spawn_blocking(move || {
        use std::io::{Read, Write};
        let mut stream = std::net::TcpStream::connect(addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut reply = String::new();
        stream.read_to_string(&mut reply).unwrap();
        let status = reply
            .split(' ')
            .nth(1)
            .and_then(|code| code.parse().ok())
            .unwrap_or(0);
        let body = reply.split_once("\r\n\r\n").map_or("", |(_, body)| body);
        (status, serde_json::from_str(body).unwrap_or(Value::Null))
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_backend_that_starts_behind_tries_once() {
    // An index made while the job had no embedder: every document waits.
    let h = Harness::new(true, off);
    h.repository().await;
    assert_eq!(h.job().await, Value::Null);
    assert!(h.waits("failing"));

    let (bound_tx, bound_rx) = std::sync::mpsc::channel();
    let server = tokio::spawn(cw_server::serve(
        config(h.data.path(), true, failing_at_start),
        "127.0.0.1:0".parse().unwrap(),
        move |addr| {
            bound_tx.send(addr).unwrap();
            Ok(())
        },
        // Never resolves: the route stops the server.
        std::future::pending(),
    ));
    let addr = tokio::task::spawn_blocking(move || bound_rx.recv().unwrap())
        .await
        .unwrap();

    // At its start the backend tries once, and the job says why it failed.
    let mut job = Value::Null;
    for _ in 0..TRIES {
        let (status, body) = request(addr, "GET", "/v1/embed/job").await;
        assert_eq!(status, 200, "{body}");
        job = body["job"].clone();
        if job["phase"] == "failed" {
            break;
        }
        tokio::time::sleep(PAUSE).await;
    }
    assert_eq!(job["phase"], "failed", "{job}");
    let error = job["error"].as_str().unwrap();
    assert!(error.contains("did not answer"), "{error}");
    assert_eq!(FAILED_AT_START.load(Ordering::SeqCst), 1);

    // Nothing tries again by itself: no retry, no new job.
    tokio::time::sleep(Duration::from_secs(2)).await;
    let (_, body) = request(addr, "GET", "/v1/embed/job").await;
    assert_eq!(body["job"], job);
    assert_eq!(FAILED_AT_START.load(Ordering::SeqCst), 1);
    assert!(h.waits("failing"));

    let (status, _) = request(addr, "POST", "/v1/shutdown").await;
    assert_eq!(status, 202);
    let ended = tokio::time::timeout(Duration::from_secs(10), server).await;
    assert!(
        matches!(ended, Ok(Ok(Ok(())))),
        "serve did not end: {ended:?}"
    );
}
