//! CLIProxyAPI: a server the user runs that holds many Claude and Codex
//! subscription logins and serves them behind one OpenAI-compatible API,
//! taking the next account for each request and resting one that reached
//! its limit (github.com/router-for-me/CLIProxyAPI).
//!
//! CodeWiki never starts, stops or installs it, and leaves its accounts to
//! it: they are added and removed in the proxy's own web panel or with its
//! `-claude-login` and `-codex-login` flags. [`CliProxy`] writes prose
//! through it with a client key; [`models`] lists what its accounts serve.
//! The shapes here were read from a live CLIProxyAPI 8.0.79 and from its
//! source.

use std::time::Duration;

use cw_core::{Error, Result};
use serde_json::Value;

use crate::provider::{
    Call, Completion, LlmProvider, Method, PROBE_SYSTEM, PROBE_TIMEOUT, PROBE_USER,
    REQUEST_TIMEOUT, Reply, chat_completion, chat_request, exchange, http_error,
};

const NAME: &str = "CLIProxyAPI";
/// How long the model list waits: it is small.
const LIST_TIMEOUT: Duration = Duration::from_secs(15);
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

/// The proxy's own control panel, where its accounts are managed. It is
/// there unless the proxy's `management.disable-control-panel` is set, and
/// asks for the proxy's `management.secret-key`.
pub fn panel(base_url: &str) -> String {
    format!("{}/management.html", base(base_url))
}

/// Whether the proxy's answer means no account can answer now:
/// - a 429 after it has tried its accounts;
/// - a 503 naming an empty pool;
/// - a 400 `model_not_found`. The proxy knows a model only through an
///   account that serves it, so this is what it says once that account is
///   removed or disabled (seen on CLIProxyAPI 8.0.79).
fn exhausted(reply: &Reply) -> Option<Error> {
    let code = reply.body["error"]["code"].as_str().unwrap_or("");
    let empty_pool = reply.status == 503 && NO_ACCOUNT.contains(&code);
    let unserved = reply.status == 400 && code == "model_not_found";
    if reply.status != 429 && !empty_pool && !unserved {
        return None;
    }
    let message = reply.body["error"]["message"]
        .as_str()
        .or_else(|| reply.body["error"].as_str())
        .unwrap_or("every account is at its limit");
    let message = if unserved {
        format!("{NAME}: {message}; no account in the proxy serves this model")
    } else {
        format!("{NAME}: {message}")
    };
    Some(Error::Exhausted {
        message,
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
        headers: &[("Authorization", &auth)],
        body: None,
        timeout: LIST_TIMEOUT,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_empty_pool_a_limit_or_an_unserved_model_is_exhaustion() {
        let reply = |status: u16, code: &str| Reply {
            status,
            retry_after: Some(5),
            body: serde_json::json!({ "error": { "message": "m", "code": code } }),
        };
        assert!(exhausted(&reply(503, "auth_unavailable")).is_some());
        assert!(exhausted(&reply(503, "auth_not_found")).is_some());
        assert!(exhausted(&reply(429, "")).is_some());
        assert!(exhausted(&reply(400, "model_not_found")).is_some());
        assert!(exhausted(&reply(503, "thread_auth_unavailable")).is_none());
        assert!(exhausted(&reply(500, "auth_unavailable")).is_none());
        assert!(exhausted(&reply(400, "invalid_request_error")).is_none());
        let Some(Error::Exhausted {
            message,
            retry_after,
        }) = exhausted(&reply(429, "model_cooldown"))
        else {
            panic!("expected Exhausted");
        };
        assert_eq!((message.as_str(), retry_after), ("CLIProxyAPI: m", Some(5)));
    }

    #[test]
    fn the_panel_is_beside_the_api() {
        assert_eq!(
            panel(" http://127.0.0.1:8317/ "),
            "http://127.0.0.1:8317/management.html"
        );
    }
}
