//! `codewiki serve`: run the backend HTTP server.

use std::net::{IpAddr, SocketAddr};
use std::path::Path;

use cw_core::{Error, Result};
use cw_server::ServerConfig;
use serde_json::json;

/// Environment variable carrying a fixed access token (used in Docker).
pub const TOKEN_ENV: &str = "CODEWIKI_TOKEN";

/// File in the data directory that tells local clients how to connect.
pub const BACKEND_FILE: &str = "backend.json";

fn random_token() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| Error::external("generating an access token", e))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Tell local clients (the app, the MCP bridge) where the backend listens.
fn publish(data_dir: &Path, addr: SocketAddr, token: &str) -> Result<()> {
    std::fs::create_dir_all(data_dir).map_err(|e| Error::io(data_dir, e))?;
    let host = if addr.ip().is_unspecified() {
        "127.0.0.1".to_string()
    } else {
        addr.ip().to_string()
    };
    let body = json!({
        "url": format!("http://{host}:{}", addr.port()),
        "token": token,
        "pid": std::process::id(),
    });
    let path = data_dir.join(BACKEND_FILE);
    std::fs::write(&path, body.to_string() + "\n").map_err(|e| Error::io(path, e))
}

/// Whether the connection file `text` was written by the process `pid`.
fn names_process(text: &str, pid: u32) -> bool {
    serde_json::from_str::<serde_json::Value>(text)
        .is_ok_and(|file| file["pid"].as_u64() == Some(u64::from(pid)))
}

/// Delete the connection file, if it is still this backend's. A backend
/// asked to stop first finishes its running requests; a client that finds it
/// not listening may by then have started the next backend, whose file must
/// stay.
fn withdraw(data_dir: &Path) {
    let path = data_dir.join(BACKEND_FILE);
    let ours =
        std::fs::read_to_string(&path).is_ok_and(|text| names_process(&text, std::process::id()));
    if ours {
        let _ = std::fs::remove_file(path);
    }
}

pub fn run(host: IpAddr, port: u16) -> Result<()> {
    let data_dir = cw_core::paths::data_dir();
    let token = match std::env::var(TOKEN_ENV).ok().filter(|t| !t.is_empty()) {
        Some(token) => token,
        // Listening beyond this machine with a token nobody chose would leave
        // the operator unable to connect, and tempts disabling auth.
        None if !host.is_loopback() => {
            return Err(Error::invalid(format!(
                "set {TOKEN_ENV} to listen on {host}; a token is required off loopback"
            )));
        }
        None => random_token()?,
    };
    let config = ServerConfig {
        data_dir: data_dir.clone(),
        token: token.clone(),
        webhook_secret: std::env::var(cw_server::WEBHOOK_SECRET_ENV)
            .ok()
            .filter(|s| !s.is_empty()),
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::external("starting the async runtime", e))?;
    runtime.block_on(async move {
        let shutdown = async {
            let _ = tokio::signal::ctrl_c().await;
        };
        cw_server::serve(
            config,
            SocketAddr::new(host, port),
            |bound| {
                publish(&data_dir, bound, &token)?;
                log::info!(
                    "codewiki backend listening on http://{bound}; data in {}; writer: {}; embedder: {}",
                    data_dir.display(),
                    writer_name(),
                    cw_wiki::semantic::embedder().map_or("none".into(), |e| e.name())
                );
                Ok(())
            },
            shutdown,
        )
        .await?;
        // Leave no stale connection file behind.
        withdraw(&data_dir);
        log::info!("codewiki backend stopped");
        Ok(())
    })
}

/// The LLM provider the environment chooses, by name; never its key.
fn writer_name() -> String {
    match cw_llm::provider::from_env() {
        Ok(Some(provider)) => provider.name(),
        Ok(None) => "none".into(),
        Err(e) => format!("not set up ({e})"),
    }
}

#[cfg(test)]
mod tests {
    use super::names_process;

    #[test]
    fn a_connection_file_belongs_to_the_process_it_names() {
        let file = r#"{"url":"http://127.0.0.1:51051","token":"t","pid":4760}"#;
        assert!(names_process(file, 4760));
        // The next backend has written the file since: it is not ours to delete.
        assert!(!names_process(file, 4761));
        assert!(!names_process("not json", 4760));
        assert!(!names_process(r#"{"url":"http://127.0.0.1:1"}"#, 4760));
    }
}
