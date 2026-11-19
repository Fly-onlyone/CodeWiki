//! The routes of the built-in embedding models, driven in-process.
//!
//! No test here runs a model, downloads one or reads this machine: the
//! backend is given a stand-in embedder, a fixed machine and a "game" that a
//! test turns on and off. The download itself is tested in `cw-models`.
//!
//! The engine is Windows only, and so are these routes.
#![cfg(windows)]

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use cw_core::Result;
use cw_embed::{Embedder, LocalEmbedder};
use cw_models::{Hardware, catalog};
use cw_server::{Host, ServerConfig, router};
use cw_wiki::semantic::behind;
use cw_wiki::service::{DOCUMENT_KINDS, open_existing_index_in};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const TOKEN: &str = "test-token";
const SMALL: &str = "granite-small-english-r2";
const GB: u64 = 1024 * 1024 * 1024;

fn no_env(_: &str) -> Option<String> {
    None
}

/// The embedder a background job is given: another one than the index was
/// made with, as far as the index can tell, and slow enough to be watched.
struct StandIn(LocalEmbedder);

impl Embedder for StandIn {
    fn name(&self) -> String {
        "stand-in".into()
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        std::thread::sleep(Duration::from_millis(60));
        self.0.embed(texts)
    }

    fn batch_size(&self) -> usize {
        1
    }
}

fn stand_in() -> Option<Arc<dyn Embedder>> {
    Some(Arc::new(StandIn(LocalEmbedder::default())))
}

/// A machine with 16 GB, 12 cores and no NVIDIA card.
fn machine(_: &Path) -> Hardware {
    Hardware {
        ram_total: 16 * GB,
        ram_available: 8 * GB,
        physical_cores: 12,
        nvidia: None,
        disk_free: Some(100 * GB),
    }
}

/// The game of the one test that plays.
static GAME: AtomicBool = AtomicBool::new(false);

fn playing() -> bool {
    GAME.load(Ordering::SeqCst)
}

fn host(game: fn() -> bool) -> Host {
    Host {
        embedder: stand_in,
        hardware: machine,
        game,
        pause_poll: Duration::from_millis(20),
    }
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Put a model of the catalog under the data directory as an install leaves
/// it, with empty files of the right sizes.
fn install_model(data: &Path, id: &str) {
    let entry = catalog::find(id).unwrap();
    let dir = cw_models::Store::new(data, None).dir(entry);
    fs::create_dir_all(&dir).unwrap();
    for file in &entry.files {
        fs::File::create(dir.join(&file.path))
            .unwrap()
            .set_len(file.size)
            .unwrap();
    }
    write(
        &dir,
        "manifest.json",
        &json!({ "revision": entry.revision }).to_string(),
    );
}

struct Harness {
    app: Router,
    data: tempfile::TempDir,
    repo: tempfile::TempDir,
}

impl Harness {
    fn new(settings_writable: bool, env: fn(&str) -> Option<String>, host: Host) -> Self {
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
        let app = router(ServerConfig {
            data_dir: data.path().to_path_buf(),
            token: TOKEN.into(),
            webhook_secret: None,
            settings_writable,
            env,
            host,
        });
        Self { app, data, repo }
    }

    fn local() -> Self {
        Self::new(true, no_env, host(|| false))
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
    async fn start(&self, uri: &str) -> Value {
        let (status, body) = self.call("POST", uri, None).await;
        assert_eq!(status, StatusCode::ACCEPTED, "POST {uri}: {body}");
        body["job"].clone()
    }

    /// The job, once `ready` holds for it.
    async fn job_when(&self, what: &str, ready: impl Fn(&Value) -> bool) -> Value {
        for _ in 0..1000 {
            let job = self.ok("GET", "/v1/models", None).await["job"].clone();
            if ready(&job) {
                return job;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!(
            "no job {what}: {}",
            self.ok("GET", "/v1/models", None).await["job"]
        );
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
    /// stand-in embedder.
    fn waits(&self) -> bool {
        let index = open_existing_index_in(self.data.path(), self.repo.path())
            .unwrap()
            .expect("the repository has an index");
        behind(&index, "stand-in", &DOCUMENT_KINDS).unwrap()
    }
}

#[tokio::test]
async fn a_backend_that_listens_beyond_this_machine_answers_no_model_route() {
    let h = Harness::new(false, no_env, host(|| false));
    for (method, uri) in [
        ("GET", "/v1/models".to_string()),
        ("POST", format!("/v1/models/{SMALL}/install")),
        ("POST", format!("/v1/models/{SMALL}/use")),
        ("DELETE", format!("/v1/models/{SMALL}")),
        ("POST", "/v1/models/cancel".to_string()),
        ("POST", "/v1/models/reembed".to_string()),
    ] {
        let (status, body) = h.call(method, &uri, None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {uri}");
        assert!(
            body["error"]
                .as_str()
                .unwrap()
                .contains("managed on its own machine"),
            "{body}"
        );
    }
    assert!(!h.data.path().join("models").exists());
}

#[tokio::test]
async fn the_models_are_listed_with_the_machine_and_the_recommendation() {
    let h = Harness::local();
    let models = h.ok("GET", "/v1/models", None).await;
    assert_eq!(models["available"], true);
    assert_eq!(models["hardware"]["physical_cores"], 12);
    assert_eq!(models["hardware"]["nvidia"], Value::Null);
    // A test program has no worker for the graphics card beside it.
    assert_eq!(models["cuda"], false);
    assert_eq!(
        models["recommended"],
        json!({
            "model": "gte-modernbert-base",
            "device": "cpu",
            "reason": "16.0 GB of memory and 12 processor cores.",
        })
    );
    assert_eq!(models["job"], Value::Null);
    let listed = models["models"].as_array().unwrap();
    let ids: Vec<&str> = listed.iter().map(|m| m["id"].as_str().unwrap()).collect();
    assert_eq!(ids, [SMALL, "gte-modernbert-base", "qwen3-embedding-0.6b"]);
    assert_eq!(
        listed[0],
        json!({
            "id": SMALL,
            "name": "Granite Embedding Small English R2",
            "tier": "small",
            "languages": "English",
            "license": "Apache-2.0",
            "dimensions": 384,
            "size": 98_916_591u64,
            "installed": false,
            "active": false,
        })
    );

    install_model(h.data.path(), SMALL);
    let models = h.ok("GET", "/v1/models", None).await;
    assert_eq!(models["models"][0]["installed"], true);
    assert_eq!(models["models"][0]["active"], false);
    assert_eq!(models["models"][1]["installed"], false);

    // A model the catalog does not have, and one that is not on the disk.
    let (status, _) = h.call("POST", "/v1/models/bert/install", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, body) = h
        .call("POST", "/v1/models/gte-modernbert-base/use", None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["error"].as_str().unwrap().contains("is not installed"),
        "{body}"
    );
    let (status, _) = h.call("DELETE", "/v1/models/bert", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // Nothing of that started a job.
    assert_eq!(h.ok("GET", "/v1/models", None).await["job"], Value::Null);
}

#[tokio::test]
async fn using_a_model_switches_the_embedder_and_embeds_every_repository_again() {
    let h = Harness::local();
    h.repository().await;
    // The update embedded with the embedder of the settings, not with the
    // one the background is given.
    assert!(h.waits());
    // That update left documents waiting, so it started a job by itself.
    let job = h.job_in("done").await;
    assert_eq!(job["kind"], "reembed");
    assert!(!h.waits());

    install_model(h.data.path(), SMALL);
    let job = h.start(&format!("/v1/models/{SMALL}/use")).await;
    assert_eq!(job["kind"], "use");
    assert_eq!(job["model"], SMALL);
    let job = h.job_in("done").await;
    assert_eq!(job["error"], Value::Null);
    assert_eq!(job["note"], Value::Null);

    let settings = h.ok("GET", "/v1/settings", None).await;
    assert_eq!(settings["embed"]["provider"]["value"], "builtin");
    assert_eq!(settings["embed"]["model"]["value"], SMALL);
    assert_eq!(
        settings["embed"]["name"],
        "builtin:granite-small-english-r2@2ab6fa8ea2d6"
    );
    let models = h.ok("GET", "/v1/models", None).await;
    assert_eq!(models["models"][0]["active"], true);

    // Deleting the model in use hands search back to the embedder that
    // needs none, and takes the files.
    let models = h.ok("DELETE", &format!("/v1/models/{SMALL}"), None).await;
    assert_eq!(models["models"][0]["installed"], false);
    assert_eq!(models["models"][0]["active"], false);
    let settings = h.ok("GET", "/v1/settings", None).await;
    assert_eq!(settings["embed"]["provider"]["value"], "local");
    assert_eq!(settings["embed"]["model"]["value"], Value::Null);
    assert!(!h.data.path().join("models").join(SMALL).exists());
}

#[tokio::test]
async fn a_job_pauses_for_a_game_can_be_cancelled_and_goes_on_later() {
    let h = Harness::new(true, no_env, host(playing));
    // A game is running from the start, so the job that the first update
    // starts by itself stops before its first document.
    GAME.store(true, Ordering::SeqCst);
    h.repository().await;
    let job = h.job_in("paused").await;
    assert_eq!(job["kind"], "reembed");
    assert_eq!(job["docs_done"], 0);
    let total = job["docs_total"].as_u64().unwrap();
    assert!(total >= 5, "{job}");
    assert_eq!(job["repo_count"], 1);
    // The tray reads this.
    let status = h.ok("GET", "/v1/status", None).await;
    assert_eq!(status["model_job"]["phase"], "paused");
    // One job at a time.
    let (status, body) = h.call("POST", "/v1/models/reembed", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("another model job is running"),
        "{body}"
    );
    // It stays paused, and embeds nothing meanwhile.
    tokio::time::sleep(Duration::from_millis(150)).await;
    let job = h.job_in("paused").await;
    assert_eq!(job["docs_done"], 0);

    // The game ends: the job goes on by itself.
    GAME.store(false, Ordering::SeqCst);
    h.job_when("with a document done", |job| {
        job["phase"] == "embedding" && job["docs_done"].as_u64() >= Some(1)
    })
    .await;

    // Cancelled on the way: what is stored stays.
    let cancelled = h.ok("POST", "/v1/models/cancel", None).await;
    assert!(cancelled["job"].is_object());
    let job = h.job_in("cancelled").await;
    let done = job["docs_done"].as_u64().unwrap();
    assert!(done >= 1 && done < total, "{job}");
    assert!(h.waits());
    assert_eq!(
        h.ok("GET", "/v1/status", None).await["model_job"],
        Value::Null
    );

    // The next job embeds the rest and no more.
    h.start("/v1/models/reembed").await;
    let job = h.job_in("done").await;
    assert_eq!(job["docs_total"].as_u64().unwrap(), total - done);
    assert_eq!(job["docs_done"], job["docs_total"]);
    assert!(!h.waits());

    // Nothing waits now: a job has nothing to do, and says so by being done.
    h.start("/v1/models/reembed").await;
    let job = h.job_in("done").await;
    assert_eq!(job["docs_total"], 0);
}

/// The address of a host that has no file at all.
static EMPTY_HOST: OnceLock<String> = OnceLock::new();

fn empty_host(name: &str) -> Option<String> {
    (name == "CODEWIKI_MODELS_URL").then(|| EMPTY_HOST.get().cloned())?
}

#[tokio::test]
async fn an_install_that_cannot_download_fails_with_the_reason() {
    use std::io::{BufRead, BufReader, Write};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    EMPTY_HOST
        .set(format!("http://{}", listener.local_addr().unwrap()))
        .unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            while reader.read_line(&mut line).is_ok_and(|read| read > 2) {
                line.clear();
            }
            let _ = reader
                .into_inner()
                .write_all(b"HTTP/1.1 404 X\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        }
    });

    let h = Harness::new(true, empty_host, host(|| false));
    let job = h.start(&format!("/v1/models/{SMALL}/install")).await;
    assert_eq!(job["kind"], "install");
    assert_eq!(job["phase"], "downloading");
    let job = h.job_in("failed").await;
    let error = job["error"].as_str().unwrap();
    assert!(error.contains("config.json"), "{error}");
    // Nothing was installed, and the settings were not touched.
    let models = h.ok("GET", "/v1/models", None).await;
    assert_eq!(models["models"][0]["installed"], false);
    assert_eq!(
        h.ok("GET", "/v1/settings", None).await["embed"]["provider"]["value"],
        "local"
    );
    // A failed job is over: another may start.
    h.start("/v1/models/reembed").await;
    h.job_in("done").await;
}

/// An environment that names the embedder.
fn embedder_from_env(name: &str) -> Option<String> {
    (name == "CODEWIKI_EMBED").then(|| "none".to_string())
}

#[tokio::test]
async fn an_embedder_named_by_the_environment_is_not_switched() {
    let h = Harness::new(true, embedder_from_env, host(|| false));
    install_model(h.data.path(), SMALL);
    h.start(&format!("/v1/models/{SMALL}/use")).await;
    let job = h.job_in("done").await;
    let note = job["note"].as_str().unwrap();
    assert!(
        note.contains("CODEWIKI_EMBED") && note.contains("not in use"),
        "{note}"
    );
    let settings = h.ok("GET", "/v1/settings", None).await;
    assert_eq!(settings["embed"]["provider"]["value"], "none");
    assert!(!h.data.path().join("settings.json").exists());
}
