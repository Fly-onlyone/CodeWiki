//! Client for the CodeWiki backend.
//!
//! Clients hold no index of their own. They find a running backend, starting
//! a local one when none is running, and send every request to it.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

/// Explicit backend address, for a backend running in Docker or on another
/// machine. Requires `CODEWIKI_TOKEN`.
const URL_ENV: &str = "CODEWIKI_URL";
const TOKEN_ENV: &str = "CODEWIKI_TOKEN";
const BACKEND_FILE: &str = "backend.json";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub struct Backend {
    url: String,
    token: String,
    agent: ureq::Agent,
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        // Error responses carry a JSON message worth showing to the agent.
        .http_status_as_error(false)
        .build()
        .into()
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

fn is_healthy(url: &str) -> bool {
    agent(Duration::from_secs(2))
        .get(&format!("{url}/v1/health"))
        .call()
        .is_ok_and(|r| r.status().is_success())
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
    /// Find a backend, starting a local one if needed.
    pub fn connect() -> Result<Self, String> {
        if let Ok(url) = std::env::var(URL_ENV)
            && !url.is_empty()
        {
            let token = std::env::var(TOKEN_ENV)
                .map_err(|_| format!("{URL_ENV} is set but {TOKEN_ENV} is not"))?;
            return Ok(Self::new(url.trim_end_matches('/').to_string(), token));
        }
        let data_dir = cw_core::paths::data_dir();
        if let Some((url, token)) = read_backend_file(&data_dir)
            && is_healthy(&url)
        {
            return Ok(Self::new(url, token));
        }
        // No live backend: a stale file would only mislead the wait below.
        let _ = std::fs::remove_file(data_dir.join(BACKEND_FILE));
        let binary = codewiki_binary();
        Command::new(&binary)
            .arg("serve")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("could not start the backend ({}): {e}", binary.display()))?;
        let started = Instant::now();
        while started.elapsed() < STARTUP_TIMEOUT {
            if let Some((url, token)) = read_backend_file(&data_dir)
                && is_healthy(&url)
            {
                return Ok(Self::new(url, token));
            }
            std::thread::sleep(Duration::from_millis(150));
        }
        Err("the backend did not start within 15 seconds".to_string())
    }

    /// Base URL of the backend this client talks to.
    pub fn url(&self) -> &str {
        &self.url
    }

    fn new(url: String, token: String) -> Self {
        Self {
            url,
            token,
            // Generation on a large repository can take minutes.
            agent: agent(Duration::from_secs(600)),
        }
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
            self.agent
                .get(&format!("{}{path}", self.url))
                .header("Authorization", &format!("Bearer {}", self.token))
                .call(),
        )
    }

    pub fn put(&self, path: &str, body: &Value) -> Result<Value, String> {
        Self::finish(
            self.agent
                .put(&format!("{}{path}", self.url))
                .header("Authorization", &format!("Bearer {}", self.token))
                .send_json(body),
        )
    }

    pub fn post(&self, path: &str, body: &Value) -> Result<Value, String> {
        Self::finish(
            self.agent
                .post(&format!("{}{path}", self.url))
                .header("Authorization", &format!("Bearer {}", self.token))
                .send_json(body),
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
}
