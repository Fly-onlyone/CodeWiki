//! CLIProxyAPI: a server the user runs that holds many Claude and Codex
//! subscription logins and serves them behind one OpenAI-compatible API,
//! taking the next account for each request and resting one that reached
//! its limit (github.com/router-for-me/CLIProxyAPI).
//!
//! CodeWiki never starts, stops or installs it. [`CliProxy`] writes prose
//! through it; [`Management`] lists, adds and removes the accounts through
//! its management API, version 8. The shapes here were read from a live
//! CLIProxyAPI 8.0.79 and from its source.

use std::net::IpAddr;
use std::time::Duration;

use cw_core::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::provider::{
    Call, Completion, LlmProvider, Method, PROBE_SYSTEM, PROBE_TIMEOUT, PROBE_USER,
    REQUEST_TIMEOUT, Reply, chat_completion, chat_request, exchange, http_error,
};

const NAME: &str = "CLIProxyAPI";
/// The management API version this client speaks.
const MANAGEMENT: &str = "/v8/management";
/// How long a management or model-list request waits: they are small.
const MANAGEMENT_TIMEOUT: Duration = Duration::from_secs(15);
/// How long [`reachable`] waits: a local server answers at once.
const REACH_TIMEOUT: Duration = Duration::from_secs(3);
/// Error codes with which the proxy says no account can take a request
/// (`isAuthSelectionUnavailable` in its `sdk/api/handlers/handlers_errors.go`).
const NO_ACCOUNT: [&str; 2] = ["auth_unavailable", "auth_not_found"];

/// What an error says when nothing answers at `base_url`.
fn unreachable(base_url: &str) -> String {
    format!("{NAME} not reachable at {}", base(base_url))
}

fn base(base_url: &str) -> &str {
    base_url.trim().trim_end_matches('/')
}

/// Whether the proxy's answer means no account can answer now: a 429 after
/// it has tried its accounts, or a 503 naming an empty pool.
fn exhausted(reply: &Reply) -> Option<Error> {
    let code = reply.body["error"]["code"].as_str().unwrap_or("");
    let empty_pool = reply.status == 503 && NO_ACCOUNT.contains(&code);
    if reply.status != 429 && !empty_pool {
        return None;
    }
    let message = reply.body["error"]["message"]
        .as_str()
        .or_else(|| reply.body["error"].as_str())
        .unwrap_or("every account is at its limit");
    Some(Error::Exhausted {
        message: format!("{NAME}: {message}"),
        retry_after: reply.retry_after,
    })
}

/// Writes prose through a CLIProxyAPI server with chat completions.
pub struct CliProxy {
    pub base_url: String,
    /// A key from the proxy's `access.api-keys`.
    pub api_key: String,
    pub model: String,
}

impl CliProxy {
    fn send(&self, system: &str, user: &str, timeout: Duration) -> Result<Value> {
        let body = chat_request(&self.model, system, user);
        let auth = format!("Bearer {}", self.api_key);
        let reply = exchange(&Call {
            provider: NAME,
            unreachable: &unreachable(&self.base_url),
            method: Method::Post,
            url: &format!("{}/v1/chat/completions", base(&self.base_url)),
            query: &[],
            headers: &[("Authorization", &auth)],
            body: Some(&body),
            timeout,
        })?;
        if let Some(error) = exhausted(&reply) {
            return Err(error);
        }
        if !reply.is_success() {
            return Err(http_error(NAME, &reply));
        }
        Ok(reply.body)
    }
}

impl LlmProvider for CliProxy {
    fn name(&self) -> String {
        format!("cliproxy:{}", self.model)
    }

    fn probe(&self) -> Result<()> {
        self.send(PROBE_SYSTEM, PROBE_USER, PROBE_TIMEOUT)
            .map(|_| ())
    }

    fn complete(&self, system: &str, user: &str) -> Result<Completion> {
        let value = self.send(system, user, REQUEST_TIMEOUT)?;
        Ok(chat_completion(&value))
    }
}

/// Whether anything answers at `base_url`: any HTTP answer will do, a
/// refused key included.
pub fn reachable(base_url: &str) -> bool {
    exchange(&Call {
        provider: NAME,
        unreachable: &unreachable(base_url),
        method: Method::Get,
        url: &format!("{}/v1/models", base(base_url)),
        query: &[],
        headers: &[],
        body: None,
        timeout: REACH_TIMEOUT,
    })
    .is_ok()
}

/// The models the proxy can serve with the accounts it holds, sorted.
pub fn models(base_url: &str, api_key: &str) -> Result<Vec<String>> {
    let auth = format!("Bearer {api_key}");
    let reply = exchange(&Call {
        provider: NAME,
        unreachable: &unreachable(base_url),
        method: Method::Get,
        url: &format!("{}/v1/models", base(base_url)),
        query: &[],
        headers: &[("Authorization", &auth)],
        body: None,
        timeout: MANAGEMENT_TIMEOUT,
    })?;
    if !reply.is_success() {
        return Err(http_error(NAME, &reply));
    }
    let mut ids: Vec<String> = reply.body["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|model| model["id"].as_str())
        .map(str::to_string)
        .collect();
    ids.sort();
    ids.dedup();
    Ok(ids)
}

/// Whether `url` names this machine. An account can be added only through
/// a proxy on the machine of the browser that signs in, because the vendor
/// sends the browser back to a port on the proxy's machine.
pub fn is_loopback(url: &str) -> bool {
    let Ok(uri) = url.trim().parse::<ureq::http::Uri>() else {
        return false;
    };
    let Some(host) = uri.host() else {
        return false;
    };
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

/// One subscription login the proxy holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// The credential file's name: what removes it.
    pub name: String,
    /// `claude`, `codex` and so on.
    pub provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// The proxy's word for its state, such as `active` or `error`.
    pub status: String,
    /// Why it is not usable, when the proxy said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Resting after a limit or a failure.
    pub unavailable: bool,
    pub disabled: bool,
    /// When a resting account is tried again (RFC 3339), when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_retry_after: Option<String>,
}

impl Account {
    fn from_entry(entry: &Value) -> Option<Self> {
        let text = |key: &str| {
            entry[key]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        };
        Some(Account {
            name: text("name")?,
            provider: text("provider")
                .or_else(|| text("type"))
                .unwrap_or_default(),
            email: text("email"),
            status: text("status").unwrap_or_default(),
            message: text("status_message"),
            unavailable: entry["unavailable"].as_bool().unwrap_or(false),
            disabled: entry["disabled"].as_bool().unwrap_or(false),
            next_retry_after: text("next_retry_after"),
        })
    }
}

/// A sign-in the proxy started: the page to open, and the state that names
/// it when asking how it went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Login {
    pub url: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginStatus {
    /// The user has not finished in the browser yet.
    Waiting,
    /// The account is saved.
    Done,
    Failed(String),
}

/// The proxy's management API, which needs its `management.secret-key`.
pub struct Management {
    pub base_url: String,
    pub key: String,
}

impl Management {
    fn call(&self, method: Method, path: &str, query: &[(&str, &str)]) -> Result<Value> {
        let auth = format!("Bearer {}", self.key);
        let reply = exchange(&Call {
            provider: NAME,
            unreachable: &unreachable(&self.base_url),
            method,
            url: &format!("{}{MANAGEMENT}{path}", base(&self.base_url)),
            query,
            headers: &[("Authorization", &auth)],
            body: None,
            timeout: MANAGEMENT_TIMEOUT,
        })?;
        if reply.is_success() {
            return Ok(reply.body);
        }
        // A route the server does not know is a plain-text 404: an older
        // proxy, or one whose management API is off.
        if reply.status == 404 && !reply.body.is_object() {
            return Err(Error::invalid(format!(
                "the {NAME} at {} has no management API: CodeWiki needs {NAME} 8 with management.secret-key set in its config.yaml",
                base(&self.base_url)
            )));
        }
        let error = http_error(NAME, &reply);
        if reply.status == 401 {
            return Err(Error::invalid(format!(
                "{error}; check the management key against management.secret-key"
            )));
        }
        Err(error)
    }

    /// Every account the proxy holds.
    pub fn accounts(&self) -> Result<Vec<Account>> {
        let body = self.call(Method::Get, "/credentials", &[])?;
        Ok(body["files"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Account::from_entry)
            .collect())
    }

    /// Start signing in to `provider` (`claude` or `codex`).
    pub fn start_login(&self, provider: &str) -> Result<Login> {
        let body = self.call(Method::Get, "/oauth/auth-url", &[("provider", provider)])?;
        let field = |key: &str| body[key].as_str().unwrap_or("").to_string();
        let login = Login {
            url: field("url"),
            state: field("state"),
        };
        if login.url.is_empty() || login.state.is_empty() {
            return Err(Error::invalid(format!(
                "{NAME} gave no sign-in page for {provider}"
            )));
        }
        Ok(login)
    }

    pub fn login_status(&self, state: &str) -> Result<LoginStatus> {
        let body = self.call(Method::Get, "/oauth/status", &[("state", state)])?;
        Ok(match body["status"].as_str().unwrap_or("") {
            "ok" => LoginStatus::Done,
            "wait" => LoginStatus::Waiting,
            _ => LoginStatus::Failed(
                body["error"]
                    .as_str()
                    .unwrap_or("the sign-in failed")
                    .to_string(),
            ),
        })
    }

    pub fn cancel_login(&self, state: &str) -> Result<()> {
        self.call(Method::Delete, "/oauth/session", &[("state", state)])
            .map(|_| ())
    }

    /// Forget the account saved as `name`.
    pub fn remove(&self, name: &str) -> Result<()> {
        self.call(Method::Delete, "/credentials", &[("name", name)])
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_addresses_are_recognised() {
        for url in [
            "http://127.0.0.1:8317",
            "http://localhost:8317/",
            "http://LOCALHOST",
            "http://[::1]:8317",
            "http://127.0.0.2:1",
        ] {
            assert!(is_loopback(url), "{url}");
        }
        for url in [
            "http://192.168.1.5:8317",
            "https://proxy.example.com",
            "not a url",
            "",
        ] {
            assert!(!is_loopback(url), "{url}");
        }
    }

    #[test]
    fn only_an_empty_pool_or_a_limit_is_exhaustion() {
        let reply = |status: u16, code: &str| Reply {
            status,
            retry_after: Some(5),
            body: serde_json::json!({ "error": { "message": "m", "code": code } }),
        };
        assert!(exhausted(&reply(503, "auth_unavailable")).is_some());
        assert!(exhausted(&reply(503, "auth_not_found")).is_some());
        assert!(exhausted(&reply(429, "")).is_some());
        assert!(exhausted(&reply(503, "thread_auth_unavailable")).is_none());
        assert!(exhausted(&reply(500, "auth_unavailable")).is_none());
        let Some(Error::Exhausted {
            message,
            retry_after,
        }) = exhausted(&reply(429, "model_cooldown"))
        else {
            panic!("expected Exhausted");
        };
        assert_eq!((message.as_str(), retry_after), ("CLIProxyAPI: m", Some(5)));
    }
}
