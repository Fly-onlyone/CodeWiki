//! Routes and handlers.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use cw_core::hash::{constant_time_eq, hmac_sha256};
use cw_core::{Error, Result, VAULT_DIR};
use cw_index::Index;
use cw_vault::{VaultMeta, proposal};
use cw_wiki::{query, service};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{AppState, models, repos, settings, workspace};

struct ApiError(Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            Error::NotFound(_) => StatusCode::NOT_FOUND,
            Error::Invalid(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        if status.is_server_error() {
            log::warn!("request failed: {}", self.0);
        }
        (status, Json(json!({ "error": self.0.to_string() }))).into_response()
    }
}

type Api<T> = std::result::Result<Json<T>, ApiError>;

/// Run blocking work (SQLite, file I/O, parsing) off the async runtime.
async fn blocking<T, F>(work: F) -> Api<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    match tokio::task::spawn_blocking(work).await {
        Ok(Ok(value)) => Ok(Json(value)),
        Ok(Err(e)) => Err(ApiError(e)),
        Err(e) => Err(ApiError(Error::external("running a request", e))),
    }
}

async fn require_token(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let presented = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    match presented {
        Some(token) if constant_time_eq(token.as_bytes(), state.0.config.token.as_bytes()) => {
            // Counted only with the token: anyone can send the header.
            let from_bridge = request
                .headers()
                .get(cw_core::CLIENT_HEADER)
                .is_some_and(|client| client.as_bytes() == cw_core::MCP_CLIENT.as_bytes());
            if from_bridge {
                state.0.activity.bridge_called();
            }
            next.run(request).await
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "missing or invalid bearer token" })),
        )
            .into_response(),
    }
}

/// A request that takes longer than this is logged at Info.
const SLOW_REQUEST: Duration = Duration::from_secs(1);

/// The level a finished request is logged at. Failures of the backend stand
/// out; refused and slow requests are kept; the rest, and the health checks
/// clients poll with, show only at Debug.
fn request_level(path: &str, status: StatusCode, elapsed: Duration) -> log::Level {
    if status.is_server_error() {
        log::Level::Warn
    } else if path == "/v1/health" {
        log::Level::Debug
    } else if status.is_client_error() || elapsed >= SLOW_REQUEST {
        log::Level::Info
    } else {
        log::Level::Debug
    }
}

/// Log each request: method, path, status and time. Never the query, the
/// headers or the body, which carry search text, notes and the token.
async fn log_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let started = Instant::now();
    let response = next.run(request).await;
    let elapsed = started.elapsed();
    let status = response.status();
    log::log!(
        request_level(&path, status, elapsed),
        "{method} {path} {} in {} ms",
        status.as_u16(),
        elapsed.as_millis()
    );
    response
}

pub fn routes(state: AppState) -> Router {
    let protected = Router::new()
        .route("/repos", get(list_repos).post(add_repo))
        .route("/repos/{id}", get(get_repo).delete(remove_repo))
        .route("/repos/{id}/update", post(update_repo))
        .route("/repos/{id}/reindex", post(reindex_repo))
        .route("/repos/{id}/status", get(repo_status))
        .route("/repos/{id}/overview", get(repo_overview))
        .route("/repos/{id}/notes", get(list_notes))
        .route("/repos/{id}/note", get(read_note).put(write_note))
        .route("/repos/{id}/files", get(list_files))
        .route("/repos/{id}/symbols", get(find_symbols))
        .route("/repos/{id}/symbol", get(symbol_detail))
        .route("/repos/{id}/context", get(context))
        .route("/repos/{id}/path", get(dependency_path))
        .route("/repos/{id}/graph", get(graph))
        .route(
            "/repos/{id}/decisions",
            get(list_decisions).post(create_decision),
        )
        .route("/repos/{id}/why", get(why))
        .route("/repos/{id}/dead-code", get(dead_code))
        .route("/repos/{id}/doc-drift", get(doc_drift))
        .route("/repos/{id}/health", get(file_health))
        .route("/repos/{id}/hotspots", get(hotspots))
        .route("/repos/{id}/history", get(history))
        .route("/repos/{id}/risk", get(file_risk))
        .route("/repos/{id}/blast-radius", get(blast_radius))
        .route("/repos/{id}/change-risk", post(change_risk))
        .route("/repos/{id}/tests", get(test_map))
        .route("/repos/{id}/impacted-tests", post(impacted_tests))
        .route("/repos/{id}/coverage", get(coverage).post(import_coverage))
        .route("/repos/{id}/security", get(security))
        .route("/repos/{id}/refactor", get(refactor))
        .route("/repos/{id}/search", get(search))
        .route("/repos/{id}/ask", post(ask))
        .route("/repos/{id}/proposals", get(list_proposals))
        .route("/repos/{id}/proposals/{pid}", get(get_proposal))
        .route(
            "/repos/{id}/proposals/{pid}/approve",
            post(approve_proposal),
        )
        .route("/repos/{id}/proposals/{pid}/reject", post(reject_proposal))
        .route("/repos/{id}/contracts", get(repo_contracts))
        .route("/workspace", get(workspace_view))
        .route("/workspace/contracts", get(workspace_contracts))
        .route("/status", get(backend_status))
        .route("/settings", get(read_settings).put(write_settings))
        .route("/settings/test/llm", post(test_llm))
        .route("/settings/test/embed", post(test_embedder))
        .route("/models", get(read_models))
        .route("/models/cancel", post(cancel_model_job))
        .route("/models/reembed", post(reembed))
        .route("/models/{id}", axum::routing::delete(remove_model))
        .route("/models/{id}/install", post(install_model))
        .route("/models/{id}/use", post(use_model))
        .route("/shutdown", post(shutdown))
        .layer(middleware::from_fn_with_state(state.clone(), require_token));
    Router::new()
        .route("/v1/health", get(health))
        // Signed by the git host, so outside the bearer-token layer.
        .route(
            "/v1/hooks/git",
            post(git_hook).layer(DefaultBodyLimit::max(WEBHOOK_BODY_LIMIT)),
        )
        .nest("/v1", protected)
        .layer(middleware::from_fn(log_request))
        .with_state(state)
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "name": "codewiki", "version": cw_core::VERSION }))
}

/// What the backend is doing: the wiki updates that are running, whoever
/// started them, and when an agent's MCP bridge last called. The app shows it
/// in its tray icon. Unlike `/v1/health` it needs the token.
async fn backend_status(State(state): State<AppState>) -> Json<Value> {
    let mut report = state.0.activity.report();
    // The job on a built-in model that runs, or null.
    report["model_job"] = state.0.jobs.status();
    Json(report)
}

/// Stop the backend once the requests already running are done. The app calls
/// this before an update replaces the programs; a client that needs a backend
/// afterwards starts a new one.
async fn shutdown(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    log::info!("asked to stop; finishing the running requests");
    state.0.stop.notify_one();
    (StatusCode::ACCEPTED, Json(json!({ "status": "stopping" })))
}

/// The settings in effect: each with its value and where that comes from. No
/// secret is in it.
async fn read_settings(State(state): State<AppState>) -> Api<Value> {
    blocking(move || Ok(settings::describe(&state))).await
}

/// Change saved settings and answer with the new description. Refused by a
/// server that listens beyond this machine (`ServerConfig::settings_writable`).
async fn write_settings(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    if !state.0.config.settings_writable {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "this backend takes its settings from its own environment variables; they cannot be changed from another machine"
            })),
        )
            .into_response();
    }
    blocking(move || settings::change(&state, &body))
        .await
        .into_response()
}

/// Send the chosen LLM one very small request and report whether it
/// answered. Always 200: a provider that refuses is reported in the body.
async fn test_llm(State(state): State<AppState>) -> Api<Value> {
    blocking(move || Ok(settings::test_llm(&state))).await
}

/// The same for the embedder of semantic search.
async fn test_embedder(State(state): State<AppState>) -> Api<Value> {
    blocking(move || Ok(settings::test_embedder(&state))).await
}

/// The answer of every model route on a backend that listens beyond this
/// machine. A model is installed on the backend's machine and takes its
/// disk, memory and graphics card; like the settings, that is decided there.
fn models_closed(state: &AppState) -> Option<Response> {
    (!state.0.config.settings_writable).then(|| {
        (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "this backend's built-in models are managed on its own machine"
            })),
        )
            .into_response()
    })
}

/// The built-in models: the machine, the recommendation, the catalog with
/// what is installed and in use, and the job that runs.
async fn read_models(State(state): State<AppState>) -> Response {
    if let Some(closed) = models_closed(&state) {
        return closed;
    }
    blocking(move || Ok(models::describe(&state)))
        .await
        .into_response()
}

/// Start a job and answer 202 with it; its progress is in `GET /v1/models`.
async fn start_model_job(state: AppState, kind: models::Kind, id: Option<String>) -> Response {
    if let Some(closed) = models_closed(&state) {
        return closed;
    }
    match blocking(move || models::start(&state, kind, id.as_deref())).await {
        Ok(job) => (StatusCode::ACCEPTED, job).into_response(),
        Err(error) => error.into_response(),
    }
}

/// Download a model, make it the embedder, and embed every repository's
/// documents again.
async fn install_model(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    start_model_job(state, models::Kind::Install, Some(id)).await
}

/// Make an installed model the embedder, and embed again.
async fn use_model(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    start_model_job(state, models::Kind::Use, Some(id)).await
}

/// Embed again what waits, with the embedder in use.
async fn reembed(State(state): State<AppState>) -> Response {
    start_model_job(state, models::Kind::Reembed, None).await
}

/// Stop the running job. What it downloaded and embedded stays, and the
/// next job goes on from there.
async fn cancel_model_job(State(state): State<AppState>) -> Response {
    if let Some(closed) = models_closed(&state) {
        return closed;
    }
    state.0.jobs.cancel();
    Json(json!({ "job": state.0.jobs.snapshot() })).into_response()
}

/// Delete a model from the disk.
async fn remove_model(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    if let Some(closed) = models_closed(&state) {
        return closed;
    }
    blocking(move || models::remove(&state, &id))
        .await
        .into_response()
}

/// Resolved repository: its root folder and its index.
struct Repo {
    root: PathBuf,
    data_dir: PathBuf,
}

impl Repo {
    fn open(state: &AppState, id: &str) -> Result<Self> {
        let data_dir = state.0.config.data_dir.clone();
        let entry = repos::get(&data_dir, id)?;
        Ok(Self {
            root: PathBuf::from(entry.path),
            data_dir,
        })
    }

    fn index(&self) -> Result<Index> {
        service::open_index_in(&self.data_dir, &self.root)
    }

    fn vault(&self) -> PathBuf {
        self.root.join(VAULT_DIR)
    }
}

/// A registered repository as `GET /v1/repos` lists it. `present` is worked
/// out on each request, on this machine, where the paths are; it is never
/// written to `repos.json`.
#[derive(Serialize)]
struct ListedRepo {
    #[serde(flatten)]
    entry: repos::RepoEntry,
    /// Whether the repository's folder still exists.
    present: bool,
}

async fn list_repos(State(state): State<AppState>) -> Api<Vec<ListedRepo>> {
    blocking(move || {
        Ok(repos::load(&state.0.config.data_dir)?
            .into_iter()
            .map(|entry| ListedRepo {
                present: std::path::Path::new(&entry.path).is_dir(),
                entry,
            })
            .collect())
    })
    .await
}

#[derive(Deserialize)]
struct AddRepo {
    path: String,
}

async fn add_repo(
    State(state): State<AppState>,
    Json(body): Json<AddRepo>,
) -> Api<repos::RepoEntry> {
    blocking(move || {
        let _guard = state.0.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        repos::add(&state.0.config.data_dir, &body.path)
    })
    .await
}

async fn get_repo(State(state): State<AppState>, Path(id): Path<String>) -> Api<repos::RepoEntry> {
    blocking(move || repos::get(&state.0.config.data_dir, &id)).await
}

async fn remove_repo(State(state): State<AppState>, Path(id): Path<String>) -> Api<Value> {
    blocking(move || {
        let _guard = state.0.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        repos::remove(&state.0.config.data_dir, &id)?;
        Ok(json!({ "removed": id }))
    })
    .await
}

/// Re-index a repository and bring its wiki up to date, holding the write
/// lock. Shared by the update route and the git webhook.
fn run_update(state: &AppState, id: &str) -> Result<cw_wiki::Report> {
    let repo = Repo::open(state, id)?;
    // Before the lock: an update that waits for another one is running too,
    // as far as whoever asked for it can tell.
    let _running = state.0.activity.begin_update(id);
    let _guard = state.0.write_lock.lock().unwrap_or_else(|p| p.into_inner());
    // The provider is read per request so a settings change needs no restart.
    let writer = cw_llm::writer_from(&settings::resolved(state))?;
    let report = service::update_in(
        &repo.data_dir,
        &repo.root,
        writer.as_ref().map(|w| w as &dyn cw_wiki::ProseWriter),
    );
    // Still under the lock: a background embedding that stores next sees
    // that the documents changed.
    models::after_update(state, &repo.root);
    report
}

async fn update_repo(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Api<cw_wiki::Report> {
    blocking(move || run_update(&state, &id)).await
}

/// Largest webhook body accepted; GitHub caps push payloads at 25 MB.
const WEBHOOK_BODY_LIMIT: usize = 25 * 1024 * 1024;

/// True when the request proves knowledge of the webhook secret: GitHub's
/// `X-Hub-Signature-256` (HMAC-SHA256 of the raw body, hex) or GitLab's
/// `X-Gitlab-Token` (the secret itself).
fn webhook_signed(secret: &str, headers: &HeaderMap, body: &[u8]) -> bool {
    let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());
    if let Some(signature) = header("x-hub-signature-256") {
        let expected: String = hmac_sha256(secret.as_bytes(), body)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        return signature.strip_prefix("sha256=").is_some_and(|hex| {
            constant_time_eq(hex.to_ascii_lowercase().as_bytes(), expected.as_bytes())
        });
    }
    header("x-gitlab-token")
        .is_some_and(|token| constant_time_eq(token.as_bytes(), secret.as_bytes()))
}

/// Folder names a push payload may refer to: GitHub's `repository.name` and
/// `repository.full_name`, GitLab's `project.name` and
/// `project.path_with_namespace`. Namespaced names contribute their last
/// segment.
fn pushed_names(payload: &Value) -> Vec<String> {
    [
        &payload["repository"]["name"],
        &payload["repository"]["full_name"],
        &payload["project"]["name"],
        &payload["project"]["path_with_namespace"],
    ]
    .into_iter()
    .filter_map(Value::as_str)
    .filter_map(|name| name.rsplit('/').next())
    .filter(|name| !name.is_empty())
    .map(str::to_lowercase)
    .collect()
}

fn hook_error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

/// Push webhook from a git host: update the wiki of the registered repository
/// whose folder is named like the pushed one. The update runs after the
/// response is sent; a host waits only a few seconds for an answer.
async fn git_hook(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    let Some(secret) = state.0.config.webhook_secret.clone() else {
        return hook_error(StatusCode::NOT_FOUND, "webhooks are not enabled");
    };
    if !webhook_signed(&secret, &headers, &body) {
        return hook_error(StatusCode::UNAUTHORIZED, "missing or invalid signature");
    }
    let Ok(payload) = serde_json::from_slice::<Value>(&body) else {
        return hook_error(StatusCode::BAD_REQUEST, "the body is not JSON");
    };
    let names = pushed_names(&payload);
    let data_dir = state.0.config.data_dir.clone();
    let matched = tokio::task::spawn_blocking(move || {
        Ok::<_, Error>(
            repos::load(&data_dir)?
                .into_iter()
                .find(|r| names.contains(&r.name.to_lowercase())),
        )
    })
    .await;
    let entry = match matched {
        Ok(Ok(Some(entry))) => entry,
        Ok(Ok(None)) => {
            return hook_error(
                StatusCode::NOT_FOUND,
                "no registered repository matches the pushed one",
            );
        }
        Ok(Err(e)) => return ApiError(e).into_response(),
        Err(e) => return ApiError(Error::external("running a request", e)).into_response(),
    };
    let id = entry.id.clone();
    // Detached on purpose: the outcome is visible in the repository's status.
    log::info!("a push to {id} starts an update");
    tokio::task::spawn_blocking(move || {
        if let Err(e) = run_update(&state, &id) {
            log::warn!("webhook update of {id} failed: {e}");
        }
    });
    (
        StatusCode::ACCEPTED,
        Json(json!({ "repo": entry.id, "status": "update started" })),
    )
        .into_response()
}

async fn workspace_view(State(state): State<AppState>) -> Api<workspace::Workspace> {
    blocking(move || workspace::workspace(&state.0.config.data_dir)).await
}

async fn workspace_contracts(
    State(state): State<AppState>,
) -> Api<workspace::WorkspaceContractsView> {
    blocking(move || workspace::workspace_contracts(&state.0.config.data_dir)).await
}

async fn repo_contracts(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Api<cw_wiki::contracts::RepoContracts> {
    blocking(move || {
        let data_dir = &state.0.config.data_dir;
        workspace::repo_contracts(data_dir, &repos::get(data_dir, &id)?)
    })
    .await
}

async fn reindex_repo(State(state): State<AppState>, Path(id): Path<String>) -> Api<Value> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        let _guard = state.0.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        let mut index = repo.index()?;
        let facts = service::reindex(&repo.root, &mut index)?;
        Ok(json!({
            "files": facts.files.len(),
            "symbols": facts.symbols.len(),
            "edges": facts.edges.len(),
            "files_with_errors": facts.files_with_errors,
        }))
    })
    .await
}

async fn repo_status(State(state): State<AppState>, Path(id): Path<String>) -> Api<query::Status> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        query::status(&repo.root, &repo.index()?)
    })
    .await
}

async fn repo_overview(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Api<query::Overview> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        query::overview(&repo.root, &repo.index()?)
    })
    .await
}

async fn list_notes(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Api<Vec<query::NoteInfo>> {
    blocking(move || query::list_notes(&Repo::open(&state, &id)?.root)).await
}

#[derive(Deserialize)]
struct NoteQuery {
    path: String,
}

async fn read_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<NoteQuery>,
) -> Api<query::NoteContent> {
    blocking(move || query::read_note(&Repo::open(&state, &id)?.root, &q.path)).await
}

#[derive(Deserialize)]
struct NoteBody {
    text: String,
}

async fn write_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<NoteQuery>,
    Json(body): Json<NoteBody>,
) -> Api<Value> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        let _guard = state.0.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        query::write_note(&repo.root, &q.path, &body.text)?;
        service::index_notes(&repo.root, &mut repo.index()?)?;
        Ok(json!({ "saved": q.path }))
    })
    .await
}

async fn list_files(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Api<Vec<cw_core::FileRecord>> {
    blocking(move || Repo::open(&state, &id)?.index()?.files()).await
}

#[derive(Deserialize)]
struct SymbolsQuery {
    #[serde(default)]
    q: String,
    file: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    50
}

async fn find_symbols(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<SymbolsQuery>,
) -> Api<Vec<cw_core::Symbol>> {
    blocking(move || {
        let index = Repo::open(&state, &id)?.index()?;
        match &q.file {
            Some(file) => index.symbols_in_file(file),
            None => index.find_symbols(&q.q, q.limit.min(500)),
        }
    })
    .await
}

#[derive(Deserialize)]
struct NameQuery {
    name: String,
}

async fn symbol_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<NameQuery>,
) -> Api<query::SymbolDetail> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        query::symbol_detail(&repo.root, &repo.index()?, &q.name)
    })
    .await
}

#[derive(Deserialize)]
struct TargetQuery {
    target: String,
}

async fn context(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<TargetQuery>,
) -> Api<query::Context> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        query::context(&repo.root, &repo.index()?, &q.target)
    })
    .await
}

#[derive(Deserialize)]
struct PathQuery {
    from: String,
    to: String,
}

async fn dependency_path(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<PathQuery>,
) -> Api<Vec<String>> {
    blocking(move || query::dependency_path(&Repo::open(&state, &id)?.index()?, &q.from, &q.to))
        .await
}

#[derive(Serialize)]
struct GraphNode {
    id: String,
    name: String,
    kind: String,
    file: String,
    x: f32,
    y: f32,
    rank: f64,
    community: usize,
}

#[derive(Serialize)]
struct GraphData {
    nodes: Vec<GraphNode>,
    /// `[source index, target index]` pairs into `nodes`.
    edges: Vec<[u32; 2]>,
}

async fn graph(State(state): State<AppState>, Path(id): Path<String>) -> Api<GraphData> {
    blocking(move || {
        let index = Repo::open(&state, &id)?.index()?;
        let symbols: std::collections::HashMap<String, cw_core::Symbol> = index
            .symbols()?
            .into_iter()
            .map(|s| (s.qualified.clone(), s))
            .collect();
        let mut position = std::collections::HashMap::new();
        let mut nodes = Vec::new();
        for (node, m) in index.metrics()? {
            let Some(symbol) = symbols.get(&node) else {
                continue;
            };
            position.insert(node.clone(), nodes.len() as u32);
            nodes.push(GraphNode {
                id: node,
                name: symbol.name.clone(),
                kind: symbol.kind.as_str().to_string(),
                file: symbol.file.clone(),
                x: m.x,
                y: m.y,
                rank: m.pagerank,
                community: m.community,
            });
        }
        let mut edges: Vec<[u32; 2]> = index
            .edges()?
            .iter()
            .filter(|e| cw_wiki::analyze::DEPENDENCY_KINDS.contains(&e.kind))
            .filter_map(|e| Some([*position.get(&e.src)?, *position.get(&e.dst)?]))
            .collect();
        edges.sort_unstable();
        edges.dedup();
        Ok(GraphData { nodes, edges })
    })
    .await
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    kind: Option<String>,
    #[serde(default = "default_search_limit")]
    limit: usize,
}

fn default_search_limit() -> usize {
    20
}

async fn search(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<SearchQuery>,
) -> Api<Vec<cw_index::Hit>> {
    blocking(move || {
        let index = Repo::open(&state, &id)?.index()?;
        cw_wiki::semantic::search(&index, &q.q, q.kind.as_deref(), q.limit.min(100))
    })
    .await
}

#[derive(Deserialize)]
struct AskBody {
    question: String,
}

async fn ask(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AskBody>,
) -> Api<query::Answer> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        let index = repo.index()?;
        // With a provider configured the answer is written from the matching
        // notes; without one it is their summaries.
        match cw_llm::from_settings(&settings::resolved(&state))? {
            Some(provider) => {
                cw_llm::answer::answer(provider.as_ref(), &repo.root, &index, &body.question)
            }
            None => query::answer(&repo.root, &index, &body.question),
        }
    })
    .await
}

#[derive(Serialize)]
struct ProposalDetail {
    proposal: proposal::Proposal,
    patch: String,
    proposed: String,
}

async fn list_proposals(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Api<Vec<proposal::Proposal>> {
    blocking(move || proposal::list(&Repo::open(&state, &id)?.vault())).await
}

async fn get_proposal(
    State(state): State<AppState>,
    Path((id, pid)): Path<(String, String)>,
) -> Api<ProposalDetail> {
    blocking(move || {
        let vault = Repo::open(&state, &id)?.vault();
        Ok(ProposalDetail {
            proposal: proposal::get(&vault, &pid)?,
            patch: proposal::patch_text(&vault, &pid)?,
            proposed: proposal::proposed_text(&vault, &pid)?,
        })
    })
    .await
}

async fn approve_proposal(
    State(state): State<AppState>,
    Path((id, pid)): Path<(String, String)>,
) -> Api<proposal::Proposal> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        let _guard = state.0.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        let vault = repo.vault();
        let done = proposal::approve(&vault, &pid)?;
        let mut meta = VaultMeta::load(&vault)?;
        if let Some(entry) = meta.notes.get_mut(&done.note_id) {
            // The human reviewed this input; it needs no further proposal.
            entry.input_hash = done.input_hash.clone();
            meta.save(&vault)?;
        }
        service::index_notes(&repo.root, &mut repo.index()?)?;
        Ok(done)
    })
    .await
}

async fn reject_proposal(
    State(state): State<AppState>,
    Path((id, pid)): Path<(String, String)>,
) -> Api<proposal::Proposal> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        let _guard = state.0.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        let vault = repo.vault();
        let done = proposal::reject(&vault, &pid)?;
        let mut meta = VaultMeta::load(&vault)?;
        if let Some(entry) = meta.notes.get_mut(&done.note_id) {
            entry.rejected_inputs.push(done.input_hash.clone());
            meta.save(&vault)?;
        }
        Ok(done)
    })
    .await
}

/// Commit history per file: authors, bus factor and change partners.
async fn history(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Api<cw_wiki::gitintel::GitIntel> {
    blocking(move || cw_wiki::gitintel::load(&Repo::open(&state, &id)?.index()?)).await
}

#[derive(Deserialize)]
struct LimitQuery {
    #[serde(default = "default_search_limit")]
    limit: usize,
}

async fn hotspots(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<LimitQuery>,
) -> Api<Vec<cw_wiki::gitintel::Hotspot>> {
    blocking(move || {
        let index = Repo::open(&state, &id)?.index()?;
        let intel = cw_wiki::gitintel::load(&index)?;
        cw_wiki::gitintel::hotspots(&index, &intel, q.limit.min(200))
    })
    .await
}

#[derive(Deserialize)]
struct FileQuery {
    path: String,
}

async fn file_risk(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<FileQuery>,
) -> Api<cw_wiki::gitintel::FileRisk> {
    blocking(move || {
        let index = Repo::open(&state, &id)?.index()?;
        let intel = cw_wiki::gitintel::load(&index)?;
        cw_wiki::gitintel::file_risk(&index, &intel, &q.path.replace('\\', "/"))
    })
    .await
}

async fn blast_radius(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<FileQuery>,
) -> Api<cw_wiki::gitintel::BlastRadius> {
    blocking(move || {
        let index = Repo::open(&state, &id)?.index()?;
        cw_wiki::gitintel::blast_radius(&index, &q.path.replace('\\', "/"))
    })
    .await
}

#[derive(Deserialize, Default)]
struct ChangeBody {
    /// Changed files; when empty, the working tree's changes are used.
    #[serde(default)]
    files: Vec<String>,
}

async fn change_risk(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<ChangeBody>>,
) -> Api<cw_wiki::gitintel::ChangeRisk> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        let index = repo.index()?;
        let intel = cw_wiki::gitintel::load(&index)?;
        let mut files = body.map(|b| b.0.files).unwrap_or_default();
        if files.is_empty() {
            files = cw_wiki::gitintel::working_tree_changes(&repo.root);
        }
        let files: Vec<String> = files.iter().map(|f| f.replace('\\', "/")).collect();
        cw_wiki::gitintel::change_risk(&index, &intel, &files)
    })
    .await
}

async fn dead_code(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Api<Vec<cw_wiki::health::DeadSymbol>> {
    blocking(move || cw_wiki::health::dead_code(&Repo::open(&state, &id)?.index()?)).await
}

async fn doc_drift(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Api<cw_wiki::health::DocReport> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        cw_wiki::health::doc_drift(&repo.root, &repo.index()?)
    })
    .await
}

async fn file_health(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Api<Vec<cw_wiki::health::FileHealth>> {
    blocking(move || cw_wiki::health::file_health(&Repo::open(&state, &id)?.index()?)).await
}

async fn list_decisions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Api<Vec<cw_wiki::decisions::Decision>> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        cw_wiki::decisions::list(&repo.root, &repo.index()?)
    })
    .await
}

#[derive(Deserialize)]
struct NewDecision {
    title: String,
}

async fn create_decision(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<NewDecision>,
) -> Api<Value> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        let _guard = state.0.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        let path = cw_wiki::decisions::create(&repo.root, &body.title)?;
        Ok(json!({ "path": path }))
    })
    .await
}

async fn why(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<TargetQuery>,
) -> Api<cw_wiki::decisions::Why> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        cw_wiki::decisions::why(&repo.root, &repo.index()?, &q.target)
    })
    .await
}

async fn test_map(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Api<cw_wiki::testmap::TestMap> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        cw_wiki::testmap::test_map(&repo.root, &repo.index()?)
    })
    .await
}

async fn impacted_tests(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<ChangeBody>>,
) -> Api<cw_wiki::testmap::Impact> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        let mut files = body.map(|b| b.0.files).unwrap_or_default();
        if files.is_empty() {
            files = cw_wiki::gitintel::working_tree_changes(&repo.root);
        }
        cw_wiki::testmap::impacted_tests(&repo.root, &repo.index()?, &files)
    })
    .await
}

/// Coverage imported earlier; without one, a report found in a conventional
/// place in the repository is read but not stored.
async fn coverage(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<LimitQuery>,
) -> Api<cw_wiki::coverage::CoverageReport> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        let index = repo.index()?;
        let data = match cw_wiki::coverage::load(&index)? {
            Some(data) => data,
            None => cw_wiki::coverage::import(&repo.root, &index, None)?,
        };
        cw_wiki::coverage::report(&index, &data, q.limit.min(200))
    })
    .await
}

#[derive(Deserialize, Default)]
struct CoverageBody {
    /// Report path relative to the repository root. When neither this nor
    /// `text` is given, conventional locations are tried.
    report: Option<String>,
    /// Report content, for a backend that cannot see the file.
    text: Option<String>,
    #[serde(default = "default_search_limit")]
    limit: usize,
}

async fn import_coverage(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<CoverageBody>>,
) -> Api<cw_wiki::coverage::CoverageReport> {
    blocking(move || {
        let repo = Repo::open(&state, &id)?;
        let body = body.map(|b| b.0).unwrap_or_default();
        let _guard = state.0.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        let index = repo.index()?;
        let data = match (&body.text, &body.report) {
            (Some(text), _) => cw_wiki::coverage::import_text(&index, text, "inline")?,
            (None, Some(report)) => {
                // The API reads files inside the repository only.
                let rel = std::path::Path::new(report);
                let inside = rel.components().all(|c| {
                    matches!(
                        c,
                        std::path::Component::Normal(_) | std::path::Component::CurDir
                    )
                });
                if !inside {
                    return Err(Error::invalid(
                        "`report` must be a path inside the repository",
                    ));
                }
                cw_wiki::coverage::import(&repo.root, &index, Some(rel))?
            }
            (None, None) => cw_wiki::coverage::import(&repo.root, &index, None)?,
        };
        cw_wiki::coverage::save(&index, &data)?;
        cw_wiki::coverage::report(&index, &data, body.limit.min(200))
    })
    .await
}

#[derive(Deserialize)]
struct SecurityQuery {
    /// Lowest level to return: `note`, `warning` or `error`.
    level: Option<String>,
}

async fn security(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<SecurityQuery>,
) -> Api<Vec<cw_wiki::security::SecurityFinding>> {
    blocking(move || {
        let floor = match q.level.as_deref() {
            None | Some("") => cw_wiki::ci::Level::Note,
            Some(text) => cw_wiki::ci::Level::parse(text).ok_or_else(|| {
                Error::invalid(format!(
                    "unknown level '{text}'; use note, warning or error"
                ))
            })?,
        };
        let repo = Repo::open(&state, &id)?;
        let mut findings = cw_wiki::security::scan(&repo.root, &repo.index()?)?;
        findings.retain(|f| f.level >= floor);
        Ok(findings)
    })
    .await
}

async fn refactor(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<LimitQuery>,
) -> Api<Vec<cw_wiki::refactor::Suggestion>> {
    blocking(move || {
        let mut plan = cw_wiki::refactor::plan(&Repo::open(&state, &id)?.index()?)?;
        plan.truncate(q.limit.min(200));
        Ok(plan)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAST: Duration = Duration::from_millis(5);

    #[test]
    fn failures_stand_out_and_routine_requests_stay_quiet() {
        assert_eq!(
            request_level(
                "/v1/repos/a1/update",
                StatusCode::INTERNAL_SERVER_ERROR,
                FAST
            ),
            log::Level::Warn
        );
        assert_eq!(
            request_level("/v1/repos", StatusCode::UNAUTHORIZED, FAST),
            log::Level::Info
        );
        assert_eq!(
            request_level("/v1/repos/a1/update", StatusCode::OK, SLOW_REQUEST),
            log::Level::Info
        );
        assert_eq!(
            request_level("/v1/repos/a1/notes", StatusCode::OK, FAST),
            log::Level::Debug
        );
        // Clients poll the health check; even a slow one is routine.
        assert_eq!(
            request_level("/v1/health", StatusCode::OK, SLOW_REQUEST),
            log::Level::Debug
        );
        assert_eq!(
            request_level("/v1/health", StatusCode::SERVICE_UNAVAILABLE, FAST),
            log::Level::Warn
        );
    }
}
