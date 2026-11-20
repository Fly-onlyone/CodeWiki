//! HTTP API of the CodeWiki backend.
//!
//! The server exposes data only; it serves no web frontend. Every route except
//! `/v1/health` requires `Authorization: Bearer <token>`. `/v1/hooks/git` is
//! the other exception: a git host cannot send the token, so it signs its
//! request with the webhook secret instead.
//!
//! `POST /v1/shutdown` stops the server. The desktop app calls it when the
//! user quits it, and before an update replaces the programs, so the old
//! backend does not keep serving.
//!
//! `GET /v1/status` says what the backend is doing: the wiki updates that are
//! running and when an agent's MCP bridge last called (`activity`).

mod activity;
mod api;
mod cliproxy;
mod models;
mod repos;
mod settings;
pub mod workspace;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use cw_core::{Error, Result};
use tokio::sync::Notify;

pub use repos::RepoEntry;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Folder for indexes and the repository list.
    pub data_dir: PathBuf,
    /// Secret every client must present.
    pub token: String,
    /// Secret a git host signs webhook calls with. `None` turns the webhook
    /// route off.
    pub webhook_secret: Option<String>,
    /// Whether a client may change the settings (`PUT /v1/settings`). False
    /// for a server that listens beyond this machine: someone holding its
    /// token could otherwise point a provider's address at their own host
    /// and be sent the API key.
    pub settings_writable: bool,
    /// Reads an environment variable for the settings, which a variable
    /// overrides: `cw_core::settings::process_env`, except in tests.
    pub env: fn(&str) -> Option<String>,
    /// What the backend asks of the machine it runs on: [`Host::default`],
    /// except in tests.
    pub host: Host,
}

/// What the work on built-in models asks of the machine. A test replaces
/// each with a stand-in, so that it starts no model, reads no hardware and
/// waits for no game.
#[derive(Debug, Clone, Copy)]
pub struct Host {
    /// The embedder that embeds documents in the background: the one the
    /// settings name.
    pub embedder: fn() -> Option<Arc<dyn cw_embed::Embedder>>,
    /// Reads the machine's memory, cores, graphics card and free disk space.
    pub hardware: fn(&Path) -> cw_models::Hardware,
    /// Whether a game is running; background embedding waits while one is.
    pub game: fn() -> bool,
    /// How often a paused job asks whether the game is over.
    pub pause_poll: Duration,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            embedder: cw_wiki::semantic::embedder,
            hardware: cw_models::hardware::probe,
            game: cw_models::game::busy,
            pause_poll: Duration::from_secs(5),
        }
    }
}

/// How long each step of an update of a wiki embeds for before it leaves the
/// rest to the background. The embedder that needs no model is through
/// thousands of documents in that time; a neural one gets a batch or two in,
/// so that the first search after an update finds something by meaning.
const INLINE_BUDGET: Duration = Duration::from_secs(1);

/// Environment variable that holds the webhook secret.
pub const WEBHOOK_SECRET_ENV: &str = "CODEWIKI_WEBHOOK_SECRET";

pub(crate) struct Inner {
    pub config: ServerConfig,
    /// Serializes writers: repository list changes and vault updates.
    pub write_lock: Mutex<()>,
    /// Serializes changes of the settings. Its own lock: an update of a wiki
    /// holds `write_lock` for minutes, and a setting must not wait for it.
    pub settings_lock: Mutex<()>,
    /// Woken by `POST /v1/shutdown`; [`serve`] stops when it fires.
    pub stop: Notify,
    /// Running updates and the last bridge call, for `GET /v1/status`.
    pub activity: activity::Activity,
    /// The job on a built-in model that runs, or the last that ran.
    pub jobs: models::Jobs,
    /// The machine as last read, for `GET /v1/models`.
    pub hardware: models::HardwareCache,
    /// Updates of a wiki finished since the start. A background embedding
    /// that sees it change starts its repository over.
    pub updates: AtomicU64,
}

#[derive(Clone)]
pub(crate) struct AppState(pub Arc<Inner>);

fn state(config: ServerConfig) -> AppState {
    AppState(Arc::new(Inner {
        config,
        write_lock: Mutex::new(()),
        settings_lock: Mutex::new(()),
        stop: Notify::new(),
        activity: activity::Activity::default(),
        jobs: models::Jobs::default(),
        hardware: models::HardwareCache::default(),
        updates: AtomicU64::new(0),
    }))
}

/// Build the router. Exposed so tests can drive it without a socket.
pub fn router(config: ServerConfig) -> Router {
    api::routes(state(config))
}

/// Serve until `shutdown` resolves or a client calls `POST /v1/shutdown`.
/// Requests already running are finished first. Returns the bound address
/// through `on_bound` so callers can publish the port when asking for port 0.
pub async fn serve(
    config: ServerConfig,
    addr: SocketAddr,
    on_bound: impl FnOnce(SocketAddr) -> Result<()>,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| Error::external(format!("binding {addr}"), e))?;
    let bound = listener
        .local_addr()
        .map_err(|e| Error::external("reading the bound address", e))?;
    on_bound(bound)?;
    let state = state(config);
    // Semantic search uses the embedder the settings name, not only the one
    // the environment names.
    cw_wiki::semantic::configure_in(&settings::resolved(&state), &state.0.config.data_dir);
    // An update holds its lock and its client's patience while it embeds;
    // what does not fit the budget is embedded in the background.
    cw_wiki::semantic::set_inline_budget(Some(INLINE_BUDGET));
    {
        let state = state.clone();
        tokio::task::spawn_blocking(move || models::resume(&state));
    }
    let inner = state.0.clone();
    let stop = async move {
        tokio::select! {
            () = shutdown => {}
            () = inner.stop.notified() => {}
        }
        // A job in the background would keep the process alive: it ends at
        // its next step. Its worker ends with the process, whose end closes
        // the worker's input.
        inner.jobs.close();
    };
    axum::serve(listener, api::routes(state))
        .with_graceful_shutdown(stop)
        .await
        .map_err(|e| Error::external("serving HTTP", e))
}
