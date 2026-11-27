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
//! running and when an agent's MCP bridge last called (`activity`), and how
//! many proposals wait in each repository (`pending`). Its `notes_rev` is a
//! counter that moves whenever a vault or a claim on a waiting note changes,
//! so a client knows when to list the waiting notes again.

mod activity;
mod api;
mod claims;
mod embed_job;
mod pending;
mod repos;
mod settings;
pub mod workspace;

use std::net::SocketAddr;
use std::path::PathBuf;
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

/// What the background embedding job asks of the machine. A test replaces
/// it with a stand-in, so that it calls no model server.
#[derive(Debug, Clone, Copy)]
pub struct Host {
    /// The embedder that embeds documents in the background: the one the
    /// settings name.
    pub embedder: fn() -> Option<Arc<dyn cw_embed::Embedder>>,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            embedder: cw_wiki::semantic::embedder,
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
    /// Waiting notes lent to agents. Lock order: `write_lock` first, this
    /// inside it, held briefly; nothing takes them the other way round.
    pub claims: Mutex<claims::Claims>,
    /// The background embedding job that runs, or the last that ran.
    pub jobs: embed_job::Jobs,
    /// The pending proposals of each repository, for `GET /v1/status`.
    pub proposals: pending::ProposalCounts,
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
        claims: Mutex::new(claims::Claims::default()),
        jobs: embed_job::Jobs::default(),
        proposals: pending::ProposalCounts::default(),
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
    cw_wiki::semantic::configure(&settings::resolved(&state));
    // An update holds its lock and its client's patience while it embeds;
    // what does not fit the budget is embedded in the background.
    cw_wiki::semantic::set_inline_budget(Some(INLINE_BUDGET));
    {
        let state = state.clone();
        tokio::task::spawn_blocking(move || {
            // Before the job looks at the repositories: a linked worktree
            // registered by an older backend is marked as one, and one
            // whose folder is gone is forgotten.
            {
                let _guard = state.0.write_lock.lock().unwrap_or_else(|p| p.into_inner());
                let dir = &state.0.config.data_dir;
                let tidied = repos::mark_worktrees(dir).and_then(|()| repos::prune_worktrees(dir));
                match tidied {
                    Ok(gone) => {
                        for entry in gone {
                            log::info!("forgot the worktree {}: its folder is gone", entry.name);
                        }
                    }
                    Err(e) => log::warn!("could not tidy the list of repositories: {e}"),
                }
            }
            embed_job::resume(&state)
        });
    }
    let inner = state.0.clone();
    let stop = async move {
        tokio::select! {
            () = shutdown => {}
            () = inner.stop.notified() => {}
        }
        // A job in the background would keep the process alive: it ends at
        // its next step.
        inner.jobs.close();
    };
    axum::serve(listener, api::routes(state))
        .with_graceful_shutdown(stop)
        .await
        .map_err(|e| Error::external("serving HTTP", e))
}
