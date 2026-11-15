//! HTTP API of the CodeWiki backend.
//!
//! The server exposes data only; it serves no web frontend. Every route except
//! `/v1/health` requires `Authorization: Bearer <token>`. `/v1/hooks/git` is
//! the other exception: a git host cannot send the token, so it signs its
//! request with the webhook secret instead.
//!
//! `POST /v1/shutdown` stops the server. The desktop app calls it before an
//! update replaces the programs, so the old backend does not keep serving.

mod api;
mod repos;
mod settings;
pub mod workspace;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
}

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
}

#[derive(Clone)]
pub(crate) struct AppState(pub Arc<Inner>);

fn state(config: ServerConfig) -> AppState {
    AppState(Arc::new(Inner {
        config,
        write_lock: Mutex::new(()),
        settings_lock: Mutex::new(()),
        stop: Notify::new(),
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
    let inner = state.0.clone();
    let stop = async move {
        tokio::select! {
            () = shutdown => {}
            () = inner.stop.notified() => {}
        }
    };
    axum::serve(listener, api::routes(state))
        .with_graceful_shutdown(stop)
        .await
        .map_err(|e| Error::external("serving HTTP", e))
}
