//! HTTP API of the CodeWiki backend.
//!
//! The server exposes data only; it serves no web frontend. Every route except
//! `/v1/health` requires `Authorization: Bearer <token>`. `/v1/hooks/git` is
//! the other exception: a git host cannot send the token, so it signs its
//! request with the webhook secret instead.

mod api;
mod repos;
pub mod workspace;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::Router;
use cw_core::{Error, Result};

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
}

/// Environment variable that holds the webhook secret.
pub const WEBHOOK_SECRET_ENV: &str = "CODEWIKI_WEBHOOK_SECRET";

pub(crate) struct Inner {
    pub config: ServerConfig,
    /// Serializes writers: repository list changes and vault updates.
    pub write_lock: Mutex<()>,
}

#[derive(Clone)]
pub(crate) struct AppState(pub Arc<Inner>);

/// Build the router. Exposed so tests can drive it without a socket.
pub fn router(config: ServerConfig) -> Router {
    api::routes(AppState(Arc::new(Inner {
        config,
        write_lock: Mutex::new(()),
    })))
}

/// Serve until `shutdown` resolves. Returns the bound address through
/// `on_bound` so callers can publish the port when asking for port 0.
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
    axum::serve(listener, router(config))
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|e| Error::external("serving HTTP", e))
}
